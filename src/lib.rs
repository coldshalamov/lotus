#![forbid(unsafe_code)]

//! Canonical Lotus integer codec.
//!
//! Lotus uses one mapping throughout the crate:
//!
//! - nonnegative payloads use `width(n) = floor(log2(n + 2))`;
//! - positive width descriptors use `width(v) = floor(log2(v + 1))`.
//!
//! Every public encoder, decoder, metric, benchmark, example, and demo is
//! derived from these two definitions.

pub mod metrics;

#[cfg(feature = "bigint")]
use num_bigint::BigUint;
#[cfg(feature = "bigint")]
use num_traits::One;
use thiserror::Error;

/// Errors emitted by Lotus codecs.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LotusError {
    #[error("the selected jumpstarter cannot represent the outer width")]
    JumpstarterOverflow,
    #[error("insufficient bits in input slice")]
    UnexpectedEof,
    #[error("invalid Lotus encoding")]
    InvalidEncoding,
    #[error("value exceeds the supported range")]
    ValueTooLarge,
}

/// A Lotus configuration: `jumpstarter_bits` fixed bits followed by `tiers`
/// positive width descriptors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LotusConfig {
    pub jumpstarter_bits: usize,
    pub tiers: usize,
}

impl LotusConfig {
    pub const fn new(jumpstarter_bits: usize, tiers: usize) -> Self {
        Self {
            jumpstarter_bits,
            tiers,
        }
    }

    pub const fn as_tuple(self) -> (usize, usize) {
        (self.jumpstarter_bits, self.tiers)
    }

    pub fn validate(self) -> Result<(), LotusError> {
        if !(1..=8).contains(&self.jumpstarter_bits) || self.tiers == 0 {
            return Err(LotusError::InvalidEncoding);
        }
        Ok(())
    }

    /// Maximum payload width described by this configuration.
    ///
    /// This is derived from the positive descriptor mapping. An `m`-bit
    /// descriptor represents positive values `2^m - 1 ..= 2^(m+1) - 2`.
    /// Values larger than `u128` are reported as `u128::MAX`.
    pub fn max_payload_width(self) -> Result<u128, LotusError> {
        self.validate()?;
        let mut width = 1u128 << self.jumpstarter_bits;
        for _ in 0..self.tiers {
            let shift = width.checked_add(1).ok_or(LotusError::ValueTooLarge)?;
            if shift >= u128::BITS as u128 {
                return Ok(u128::MAX);
            }
            width = (1u128 << shift as u32) - 2;
        }
        Ok(width)
    }

    /// Maximum `u64` value supported by this configuration.
    pub fn max_u64_value(self) -> Result<u64, LotusError> {
        let width = self.max_payload_width()?;
        if width >= u64::BITS as u128 {
            return Ok(u64::MAX);
        }
        let end = (1u128 << (width as u32 + 1)) - 3;
        u64::try_from(end).map_err(|_| LotusError::ValueTooLarge)
    }
}

/// A named, Pareto-useful Lotus configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LotusProfile {
    pub label: &'static str,
    pub config: LotusConfig,
    pub purpose: &'static str,
}

/// Minimum-overhead profile for values `0..=125`.
pub const LOTUS_TINY: LotusConfig = LotusConfig::new(1, 1);
/// One-tier density profile for values `0..=2^31-3`.
pub const LOTUS_COMPACT_31: LotusConfig = LotusConfig::new(2, 1);
/// Minimum-bit profile covering the complete `u64` domain.
pub const LOTUS_DENSE_U64: LotusConfig = LotusConfig::new(1, 2);
/// One-tier profile covering `u64`; same size as J1D2 for ordinary large values,
/// with one fewer descriptor to decode.
pub const LOTUS_FAST_U64: LotusConfig = LotusConfig::new(3, 1);

/// Recommended configurations on the size/range/latency frontier.
///
/// Dominated configurations are intentionally omitted from docs, benchmarks,
/// generated artifacts, and the interactive demo.
pub const RECOMMENDED_PROFILES: &[LotusProfile] = &[
    LotusProfile {
        label: "J1D1",
        config: LOTUS_TINY,
        purpose: "minimum overhead through 125",
    },
    LotusProfile {
        label: "J2D1",
        config: LOTUS_COMPACT_31,
        purpose: "one-tier density through 2^31-3",
    },
    LotusProfile {
        label: "J1D2",
        config: LOTUS_DENSE_U64,
        purpose: "minimum bits across the full u64 domain",
    },
    LotusProfile {
        label: "J3D1",
        config: LOTUS_FAST_U64,
        purpose: "one-tier full-u64 fast path",
    },
];

/// Backward-compatible tuple presets.
pub const LOTUS_J1D1: (usize, usize) = (1, 1);
pub const LOTUS_J2D1: (usize, usize) = (2, 1);
pub const LOTUS_J1D2: (usize, usize) = (1, 2);
pub const LOTUS_J3D1: (usize, usize) = (3, 1);

/// Encoded Lotus payload with explicit framing metadata.
///
/// Lotus is bit-oriented. `bytes` pads the final byte with zeroes, while
/// `bit_len` records the exact number of meaningful bits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedLotus {
    pub bytes: Vec<u8>,
    pub bit_len: usize,
}

/// Streaming MSB-first bit writer.
#[derive(Debug, Default, Clone)]
pub struct BitWriter {
    buffer: Vec<u8>,
    pending: u8,
    pending_bits: u8,
}

impl BitWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity_bits(bits: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(bits.div_ceil(8)),
            pending: 0,
            pending_bits: 0,
        }
    }

    pub fn bits_written(&self) -> usize {
        self.buffer.len() * 8 + self.pending_bits as usize
    }

    pub fn write_bits(&mut self, value: u64, mut width: usize) -> Result<(), LotusError> {
        if width > u64::BITS as usize {
            return Err(LotusError::ValueTooLarge);
        }
        if width == 0 {
            return if value == 0 {
                Ok(())
            } else {
                Err(LotusError::InvalidEncoding)
            };
        }
        if width < u64::BITS as usize && value >= (1u64 << width) {
            return Err(LotusError::InvalidEncoding);
        }

        while width > 0 {
            let available = 8usize - self.pending_bits as usize;
            let take = available.min(width);
            let shift = width - take;
            let mask = (1u16 << take) - 1;
            let part = ((value >> shift) as u16 & mask) as u8;

            let combined = (u16::from(self.pending) << take) | u16::from(part);
            self.pending = (combined & 0xff) as u8;
            self.pending_bits += take as u8;
            width -= take;

            if self.pending_bits == 8 {
                self.buffer.push(self.pending);
                self.pending = 0;
                self.pending_bits = 0;
            }
        }
        Ok(())
    }

    pub fn into_bytes(mut self) -> Vec<u8> {
        if self.pending_bits != 0 {
            self.buffer
                .push(self.pending << (8usize - self.pending_bits as usize));
        }
        self.buffer
    }
}

/// Streaming MSB-first bit reader.
#[derive(Clone, Debug)]
pub struct BitReader<'a> {
    bytes: &'a [u8],
    byte_pos: usize,
    current: u8,
    remaining: u8,
}

impl<'a> BitReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            byte_pos: 0,
            current: 0,
            remaining: 0,
        }
    }

    pub fn read_bits(&mut self, mut width: usize) -> Result<u64, LotusError> {
        if width > u64::BITS as usize {
            return Err(LotusError::ValueTooLarge);
        }

        let mut value = 0u64;
        while width > 0 {
            if self.remaining == 0 {
                self.current = *self
                    .bytes
                    .get(self.byte_pos)
                    .ok_or(LotusError::UnexpectedEof)?;
                self.byte_pos += 1;
                self.remaining = 8;
            }

            let take = (self.remaining as usize).min(width);
            let shift = self.remaining as usize - take;
            let mask = ((1u16 << take) - 1) as u8;
            let part = (self.current >> shift) & mask;

            value = (value << take) | u64::from(part);
            self.remaining -= take as u8;
            width -= take;
        }
        Ok(value)
    }

    pub fn bits_consumed(&self) -> usize {
        self.byte_pos * 8 - self.remaining as usize
    }
}

#[inline]
fn floor_log2_u128(value: u128) -> Result<usize, LotusError> {
    if value == 0 {
        return Err(LotusError::InvalidEncoding);
    }
    Ok((u128::BITS - 1 - value.leading_zeros()) as usize)
}

/// Canonical nonnegative Lotus width: `floor(log2(value + 2))`.
#[inline]
fn nonnegative_width(value: u64) -> usize {
    let shifted = value as u128 + 2;
    (u128::BITS - 1 - shifted.leading_zeros()) as usize
}

/// Canonical positive descriptor width: `floor(log2(value + 1))`.
#[inline]
fn positive_width(value: usize) -> Result<usize, LotusError> {
    if value == 0 {
        return Err(LotusError::InvalidEncoding);
    }
    let shifted = (value as u128)
        .checked_add(1)
        .ok_or(LotusError::ValueTooLarge)?;
    floor_log2_u128(shifted)
}

#[inline]
fn encode_nonnegative_fixed(value: u64, width: usize) -> Result<u64, LotusError> {
    if width == 0 || width > u64::BITS as usize {
        return Err(LotusError::ValueTooLarge);
    }
    let base = 1u128 << width;
    let start = base - 2;
    let payload = (value as u128)
        .checked_sub(start)
        .ok_or(LotusError::ValueTooLarge)?;
    if payload >= base {
        return Err(LotusError::ValueTooLarge);
    }
    u64::try_from(payload).map_err(|_| LotusError::ValueTooLarge)
}

#[inline]
fn decode_nonnegative_fixed(payload: u64, width: usize) -> Result<u64, LotusError> {
    if width == 0 || width > u64::BITS as usize {
        return Err(LotusError::ValueTooLarge);
    }
    let base = 1u128 << width;
    if payload as u128 >= base {
        return Err(LotusError::InvalidEncoding);
    }
    let value = (base - 2) + payload as u128;
    u64::try_from(value).map_err(|_| LotusError::ValueTooLarge)
}

#[inline]
fn encode_positive_fixed(value: usize, width: usize) -> Result<u64, LotusError> {
    if value == 0 || width == 0 || width > u64::BITS as usize {
        return Err(LotusError::InvalidEncoding);
    }
    let base = 1u128 << width;
    let start = base - 1;
    let payload = (value as u128)
        .checked_sub(start)
        .ok_or(LotusError::ValueTooLarge)?;
    if payload >= base {
        return Err(LotusError::ValueTooLarge);
    }
    u64::try_from(payload).map_err(|_| LotusError::ValueTooLarge)
}

#[inline]
fn decode_positive_fixed(payload: u64, width: usize) -> Result<usize, LotusError> {
    if width == 0 || width > u64::BITS as usize {
        return Err(LotusError::ValueTooLarge);
    }
    let base = 1u128 << width;
    if payload as u128 >= base {
        return Err(LotusError::InvalidEncoding);
    }
    let value = (base - 1) + payload as u128;
    usize::try_from(value).map_err(|_| LotusError::ValueTooLarge)
}

#[derive(Debug, Clone)]
struct WidthChain {
    /// `[payload_width, descriptor_1_width, ..., outer_descriptor_width]`.
    widths: Vec<usize>,
    total_bits: usize,
}

fn build_width_chain(value: u64, config: LotusConfig) -> Result<WidthChain, LotusError> {
    config.validate()?;

    let mut widths = Vec::with_capacity(config.tiers + 1);
    let mut width = nonnegative_width(value);
    widths.push(width);

    for _ in 0..config.tiers {
        width = positive_width(width)?;
        widths.push(width);
    }

    let jump_capacity = 1usize << config.jumpstarter_bits;
    if width > jump_capacity {
        return Err(LotusError::JumpstarterOverflow);
    }

    let total_bits = widths
        .iter()
        .try_fold(config.jumpstarter_bits, |sum, &part| {
            sum.checked_add(part).ok_or(LotusError::ValueTooLarge)
        })?;

    Ok(WidthChain { widths, total_bits })
}

fn write_width_chain(
    value: u64,
    config: LotusConfig,
    chain: &WidthChain,
    writer: &mut BitWriter,
) -> Result<(), LotusError> {
    let outer_width = *chain.widths.last().ok_or(LotusError::InvalidEncoding)?;
    writer.write_bits(
        u64::try_from(outer_width - 1).map_err(|_| LotusError::ValueTooLarge)?,
        config.jumpstarter_bits,
    )?;

    for level in (1..chain.widths.len()).rev() {
        let field_width = chain.widths[level];
        let described_width = chain.widths[level - 1];
        let payload = encode_positive_fixed(described_width, field_width)?;
        writer.write_bits(payload, field_width)?;
    }

    let payload_width = chain.widths[0];
    writer.write_bits(
        encode_nonnegative_fixed(value, payload_width)?,
        payload_width,
    )?;
    Ok(())
}

fn encode_into_writer_config(
    value: u64,
    config: LotusConfig,
    writer: &mut BitWriter,
) -> Result<usize, LotusError> {
    let chain = build_width_chain(value, config)?;
    let before = writer.bits_written();
    write_width_chain(value, config, &chain, writer)?;
    debug_assert_eq!(writer.bits_written() - before, chain.total_bits);
    Ok(chain.total_bits)
}

fn decode_from_reader_config(
    reader: &mut BitReader<'_>,
    config: LotusConfig,
) -> Result<(u64, usize), LotusError> {
    config.validate()?;
    let before = reader.bits_consumed();

    let jump = reader.read_bits(config.jumpstarter_bits)?;
    let mut next_width =
        usize::try_from(jump.checked_add(1).ok_or(LotusError::ValueTooLarge)?)
            .map_err(|_| LotusError::ValueTooLarge)?;

    for _ in 0..config.tiers {
        if next_width == 0 || next_width > u64::BITS as usize {
            return Err(LotusError::ValueTooLarge);
        }
        let payload = reader.read_bits(next_width)?;
        next_width = decode_positive_fixed(payload, next_width)?;
        if next_width == 0 || next_width > u64::BITS as usize {
            return Err(LotusError::ValueTooLarge);
        }
    }

    let payload = reader.read_bits(next_width)?;
    let value = decode_nonnegative_fixed(payload, next_width)?;
    Ok((value, reader.bits_consumed() - before))
}

/// Encode a `u64`, returning a zero-padded byte buffer.
///
/// Use [`lotus_encode_u64_framed`] or the streaming API when exact framing
/// matters.
pub fn lotus_encode_u64(value: u64, j_bits: usize, tiers: usize) -> Result<Vec<u8>, LotusError> {
    Ok(lotus_encode_u64_framed(value, j_bits, tiers)?.bytes)
}

/// Encode a `u64` and return its exact meaningful bit length.
pub fn lotus_encode_u64_framed(
    value: u64,
    j_bits: usize,
    tiers: usize,
) -> Result<EncodedLotus, LotusError> {
    let config = LotusConfig::new(j_bits, tiers);
    let chain = build_width_chain(value, config)?;
    let mut writer = BitWriter::with_capacity_bits(chain.total_bits);
    write_width_chain(value, config, &chain, &mut writer)?;
    Ok(EncodedLotus {
        bytes: writer.into_bytes(),
        bit_len: chain.total_bits,
    })
}

/// Decode one Lotus codeword and return `(value, meaningful_bits_consumed)`.
pub fn lotus_decode_u64(
    bytes: &[u8],
    j_bits: usize,
    tiers: usize,
) -> Result<(u64, usize), LotusError> {
    let mut reader = BitReader::new(bytes);
    decode_from_reader_config(&mut reader, LotusConfig::new(j_bits, tiers))
}

/// Compute the exact Lotus bit length without allocating.
pub fn lotus_encoded_bit_len(
    value: u64,
    j_bits: usize,
    tiers: usize,
) -> Result<usize, LotusError> {
    Ok(build_width_chain(value, LotusConfig::new(j_bits, tiers))?.total_bits)
}

/// Append one codeword to a shared packed bitstream.
pub fn lotus_encode_into_writer(
    value: u64,
    j_bits: usize,
    tiers: usize,
    writer: &mut BitWriter,
) -> Result<usize, LotusError> {
    encode_into_writer_config(value, LotusConfig::new(j_bits, tiers), writer)
}

/// Decode one codeword from a shared packed bitstream.
pub fn lotus_decode_from_reader(
    reader: &mut BitReader<'_>,
    j_bits: usize,
    tiers: usize,
) -> Result<(u64, usize), LotusError> {
    decode_from_reader_config(reader, LotusConfig::new(j_bits, tiers))
}

#[cfg(feature = "bigint")]
fn nonnegative_width_biguint(value: &BigUint) -> Result<usize, LotusError> {
    let shifted = value + 2u8;
    let bits = shifted.bits();
    usize::try_from(bits.checked_sub(1).ok_or(LotusError::InvalidEncoding)?)
        .map_err(|_| LotusError::ValueTooLarge)
}

#[cfg(feature = "bigint")]
fn write_biguint_bits(
    writer: &mut BitWriter,
    value: &BigUint,
    width: usize,
) -> Result<(), LotusError> {
    let bit_len = usize::try_from(value.bits()).map_err(|_| LotusError::ValueTooLarge)?;
    if bit_len > width {
        return Err(LotusError::InvalidEncoding);
    }

    let mut leading_zeroes = width - bit_len;
    while leading_zeroes > 0 {
        let chunk = leading_zeroes.min(u64::BITS as usize);
        writer.write_bits(0, chunk)?;
        leading_zeroes -= chunk;
    }

    if bit_len == 0 {
        return Ok(());
    }

    let bytes = value.to_bytes_be();
    let first_width = bit_len % 8;
    let mut index = 0;
    if first_width != 0 {
        writer.write_bits(
            u64::from(bytes[0] & ((1u8 << first_width) - 1)),
            first_width,
        )?;
        index = 1;
    }
    for &byte in &bytes[index..] {
        writer.write_bits(u64::from(byte), 8)?;
    }
    Ok(())
}

/// Encode an arbitrary-precision nonnegative integer using the same canonical
/// mapping as the `u64` implementation.
#[cfg(feature = "bigint")]
pub fn lotus_encode_biguint(
    value: &BigUint,
    j_bits: usize,
    tiers: usize,
) -> Result<Vec<u8>, LotusError> {
    let config = LotusConfig::new(j_bits, tiers);
    config.validate()?;

    let mut widths = Vec::with_capacity(tiers + 1);
    let mut width = nonnegative_width_biguint(value)?;
    widths.push(width);
    for _ in 0..tiers {
        width = positive_width(width)?;
        widths.push(width);
    }

    if width > (1usize << j_bits) {
        return Err(LotusError::JumpstarterOverflow);
    }

    let total_bits = widths.iter().try_fold(j_bits, |sum, &part| {
        sum.checked_add(part).ok_or(LotusError::ValueTooLarge)
    })?;
    let mut writer = BitWriter::with_capacity_bits(total_bits);
    writer.write_bits((width - 1) as u64, j_bits)?;

    for level in (1..widths.len()).rev() {
        let field_width = widths[level];
        let described_width = widths[level - 1];
        writer.write_bits(
            encode_positive_fixed(described_width, field_width)?,
            field_width,
        )?;
    }

    let payload_width = widths[0];
    let start = (BigUint::one() << payload_width) - 2u8;
    if value < &start {
        return Err(LotusError::ValueTooLarge);
    }
    let payload = value - start;
    write_biguint_bits(&mut writer, &payload, payload_width)?;
    Ok(writer.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_width_examples() {
        assert_eq!(nonnegative_width(0), 1);
        assert_eq!(nonnegative_width(1), 1);
        assert_eq!(nonnegative_width(2), 2);
        assert_eq!(nonnegative_width(5), 2);
        assert_eq!(nonnegative_width(6), 3);

        assert_eq!(positive_width(1), Ok(1));
        assert_eq!(positive_width(2), Ok(1));
        assert_eq!(positive_width(3), Ok(2));
        assert_eq!(positive_width(6), Ok(2));
        assert_eq!(positive_width(7), Ok(3));
    }

    #[test]
    fn profile_ranges_are_derived_from_descriptor_mapping() {
        assert_eq!(LOTUS_TINY.max_payload_width(), Ok(6));
        assert_eq!(LOTUS_TINY.max_u64_value(), Ok(125));
        assert_eq!(LOTUS_COMPACT_31.max_payload_width(), Ok(30));
        assert_eq!(LOTUS_COMPACT_31.max_u64_value(), Ok((1u64 << 31) - 3));
        assert_eq!(LOTUS_DENSE_U64.max_payload_width(), Ok(126));
        assert_eq!(LOTUS_DENSE_U64.max_u64_value(), Ok(u64::MAX));
        assert_eq!(LOTUS_FAST_U64.max_payload_width(), Ok(510));
        assert_eq!(LOTUS_FAST_U64.max_u64_value(), Ok(u64::MAX));
    }

    #[test]
    fn positive_fixed_mapping_is_dense_and_consecutive() {
        for width in 1..=12 {
            let count = 1u64 << width;
            let start = (1u64 << width) - 1;
            for payload in 0..count {
                let value = decode_positive_fixed(payload, width).unwrap();
                assert_eq!(value as u64, start + payload);
                assert_eq!(encode_positive_fixed(value, width).unwrap(), payload);
            }
        }
    }

    #[test]
    fn nonnegative_fixed_mapping_is_dense_and_consecutive() {
        for width in 1..=12 {
            let count = 1u64 << width;
            let start = (1u64 << width) - 2;
            for payload in 0..count {
                let value = decode_nonnegative_fixed(payload, width).unwrap();
                assert_eq!(value, start + payload);
                assert_eq!(encode_nonnegative_fixed(value, width).unwrap(), payload);
            }
        }
    }
}
