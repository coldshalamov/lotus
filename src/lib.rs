#![forbid(unsafe_code)]

//! Canonical Lotus integer codec.
//!
//! Payloads use `floor(log2(n + 2))`. Positive width descriptors use
//! `floor(log2(v + 1))`. Every API in this crate delegates to that format.

pub mod metrics;

#[cfg(feature = "bigint")]
use num_bigint::BigUint;
#[cfg(feature = "bigint")]
use num_traits::One;
use thiserror::Error;

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

    /// Derived from the positive descriptor range
    /// `2^w - 1 ..= 2^(w+1) - 2`.
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

    pub fn max_u64_value(self) -> Result<u64, LotusError> {
        let width = self.max_payload_width()?;
        if width >= u64::BITS as u128 {
            return Ok(u64::MAX);
        }
        u64::try_from((1u128 << (width as u32 + 1)) - 3)
            .map_err(|_| LotusError::ValueTooLarge)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LotusProfile {
    pub label: &'static str,
    pub config: LotusConfig,
    pub purpose: &'static str,
}

pub const LOTUS_TINY: LotusConfig = LotusConfig::new(1, 1);
pub const LOTUS_COMPACT_31: LotusConfig = LotusConfig::new(2, 1);
pub const LOTUS_DENSE_U64: LotusConfig = LotusConfig::new(1, 2);
pub const LOTUS_FAST_U64: LotusConfig = LotusConfig::new(3, 1);

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

pub const LOTUS_J1D1: (usize, usize) = (1, 1);
pub const LOTUS_J2D1: (usize, usize) = (2, 1);
pub const LOTUS_J1D2: (usize, usize) = (1, 2);
pub const LOTUS_J3D1: (usize, usize) = (3, 1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedLotus {
    pub bytes: Vec<u8>,
    pub bit_len: usize,
}

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
            let take = (8usize - self.pending_bits as usize).min(width);
            let shift = width - take;
            let part = ((value >> shift) & ((1u64 << take) - 1)) as u8;
            let combined = (u16::from(self.pending) << take) | u16::from(part);
            self.pending = combined as u8;
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
                .push(self.pending << (8 - self.pending_bits));
        }
        self.buffer
    }
}

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
            let part = (self.current >> shift) & (((1u16 << take) - 1) as u8);
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
fn nonnegative_width(value: u64) -> usize {
    let shifted = value as u128 + 2;
    (u128::BITS - 1 - shifted.leading_zeros()) as usize
}

#[inline]
fn positive_width(value: usize) -> Result<usize, LotusError> {
    if value == 0 {
        return Err(LotusError::InvalidEncoding);
    }
    let shifted = (value as u128)
        .checked_add(1)
        .ok_or(LotusError::ValueTooLarge)?;
    Ok((u128::BITS - 1 - shifted.leading_zeros()) as usize)
}

#[inline]
fn encode_nonnegative_fixed(value: u64, width: usize) -> Result<u64, LotusError> {
    if width == 0 || width > u64::BITS as usize {
        return Err(LotusError::ValueTooLarge);
    }
    let base = 1u128 << width;
    let payload = (value as u128)
        .checked_sub(base - 2)
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
    u64::try_from(base - 2 + payload as u128).map_err(|_| LotusError::ValueTooLarge)
}

#[inline]
fn encode_positive_fixed(value: usize, width: usize) -> Result<u64, LotusError> {
    if value == 0 || width == 0 || width > u64::BITS as usize {
        return Err(LotusError::InvalidEncoding);
    }
    let base = 1u128 << width;
    let payload = (value as u128)
        .checked_sub(base - 1)
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
    usize::try_from(base - 1 + payload as u128).map_err(|_| LotusError::ValueTooLarge)
}

#[derive(Debug)]
struct WidthChain {
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
    if width > (1usize << config.jumpstarter_bits) {
        return Err(LotusError::JumpstarterOverflow);
    }
    let total_bits = widths
        .iter()
        .try_fold(config.jumpstarter_bits, |sum, width| {
            sum.checked_add(*width).ok_or(LotusError::ValueTooLarge)
        })?;
    Ok(WidthChain { widths, total_bits })
}

fn write_codeword(
    value: u64,
    config: LotusConfig,
    chain: &WidthChain,
    writer: &mut BitWriter,
) -> Result<(), LotusError> {
    let outer = *chain.widths.last().ok_or(LotusError::InvalidEncoding)?;
    writer.write_bits((outer - 1) as u64, config.jumpstarter_bits)?;
    for level in (1..chain.widths.len()).rev() {
        let field_width = chain.widths[level];
        let described_width = chain.widths[level - 1];
        writer.write_bits(
            encode_positive_fixed(described_width, field_width)?,
            field_width,
        )?;
    }
    let payload_width = chain.widths[0];
    writer.write_bits(
        encode_nonnegative_fixed(value, payload_width)?,
        payload_width,
    )
}

fn encode_into(
    value: u64,
    config: LotusConfig,
    writer: &mut BitWriter,
) -> Result<usize, LotusError> {
    let chain = build_width_chain(value, config)?;
    write_codeword(value, config, &chain, writer)?;
    Ok(chain.total_bits)
}

fn decode_from(
    reader: &mut BitReader<'_>,
    config: LotusConfig,
) -> Result<(u64, usize), LotusError> {
    config.validate()?;
    let before = reader.bits_consumed();
    let mut width = usize::try_from(
        reader
            .read_bits(config.jumpstarter_bits)?
            .checked_add(1)
            .ok_or(LotusError::ValueTooLarge)?,
    )
    .map_err(|_| LotusError::ValueTooLarge)?;

    for _ in 0..config.tiers {
        if width == 0 || width > u64::BITS as usize {
            return Err(LotusError::ValueTooLarge);
        }
        width = decode_positive_fixed(reader.read_bits(width)?, width)?;
        if width == 0 || width > u64::BITS as usize {
            return Err(LotusError::ValueTooLarge);
        }
    }

    let value = decode_nonnegative_fixed(reader.read_bits(width)?, width)?;
    Ok((value, reader.bits_consumed() - before))
}

pub fn lotus_encode_u64(value: u64, j_bits: usize, tiers: usize) -> Result<Vec<u8>, LotusError> {
    Ok(lotus_encode_u64_framed(value, j_bits, tiers)?.bytes)
}

pub fn lotus_encode_u64_framed(
    value: u64,
    j_bits: usize,
    tiers: usize,
) -> Result<EncodedLotus, LotusError> {
    let config = LotusConfig::new(j_bits, tiers);
    let chain = build_width_chain(value, config)?;
    let mut writer = BitWriter::with_capacity_bits(chain.total_bits);
    write_codeword(value, config, &chain, &mut writer)?;
    Ok(EncodedLotus {
        bytes: writer.into_bytes(),
        bit_len: chain.total_bits,
    })
}

pub fn lotus_decode_u64(
    bytes: &[u8],
    j_bits: usize,
    tiers: usize,
) -> Result<(u64, usize), LotusError> {
    decode_from(
        &mut BitReader::new(bytes),
        LotusConfig::new(j_bits, tiers),
    )
}

pub fn lotus_encoded_bit_len(
    value: u64,
    j_bits: usize,
    tiers: usize,
) -> Result<usize, LotusError> {
    Ok(build_width_chain(value, LotusConfig::new(j_bits, tiers))?.total_bits)
}

pub fn lotus_encode_into_writer(
    value: u64,
    j_bits: usize,
    tiers: usize,
    writer: &mut BitWriter,
) -> Result<usize, LotusError> {
    encode_into(value, LotusConfig::new(j_bits, tiers), writer)
}

pub fn lotus_decode_from_reader(
    reader: &mut BitReader<'_>,
    j_bits: usize,
    tiers: usize,
) -> Result<(u64, usize), LotusError> {
    decode_from(reader, LotusConfig::new(j_bits, tiers))
}

#[cfg(feature = "bigint")]
fn big_width(value: &BigUint) -> Result<usize, LotusError> {
    usize::try_from((value + 2u8).bits() - 1).map_err(|_| LotusError::ValueTooLarge)
}

#[cfg(feature = "bigint")]
fn write_big_bits(
    writer: &mut BitWriter,
    value: &BigUint,
    width: usize,
) -> Result<(), LotusError> {
    let bit_len = usize::try_from(value.bits()).map_err(|_| LotusError::ValueTooLarge)?;
    if bit_len > width {
        return Err(LotusError::InvalidEncoding);
    }
    let mut zeroes = width - bit_len;
    while zeroes > 0 {
        let chunk = zeroes.min(u64::BITS as usize);
        writer.write_bits(0, chunk)?;
        zeroes -= chunk;
    }
    if bit_len == 0 {
        return Ok(());
    }
    let bytes = value.to_bytes_be();
    let first = bit_len % 8;
    let mut index = 0;
    if first != 0 {
        writer.write_bits(u64::from(bytes[0] & ((1u8 << first) - 1)), first)?;
        index = 1;
    }
    for &byte in &bytes[index..] {
        writer.write_bits(u64::from(byte), 8)?;
    }
    Ok(())
}

#[cfg(feature = "bigint")]
pub fn lotus_encode_biguint(
    value: &BigUint,
    j_bits: usize,
    tiers: usize,
) -> Result<Vec<u8>, LotusError> {
    let config = LotusConfig::new(j_bits, tiers);
    config.validate()?;
    let mut widths = Vec::with_capacity(tiers + 1);
    let mut width = big_width(value)?;
    widths.push(width);
    for _ in 0..tiers {
        width = positive_width(width)?;
        widths.push(width);
    }
    if width > (1usize << j_bits) {
        return Err(LotusError::JumpstarterOverflow);
    }

    let total = widths.iter().try_fold(j_bits, |sum, width| {
        sum.checked_add(*width).ok_or(LotusError::ValueTooLarge)
    })?;
    let mut writer = BitWriter::with_capacity_bits(total);
    writer.write_bits((width - 1) as u64, j_bits)?;
    for level in (1..widths.len()).rev() {
        writer.write_bits(
            encode_positive_fixed(widths[level - 1], widths[level])?,
            widths[level],
        )?;
    }

    let payload_width = widths[0];
    let start = (BigUint::one() << payload_width) - 2u8;
    if value < &start {
        return Err(LotusError::ValueTooLarge);
    }
    write_big_bits(&mut writer, &(value - start), payload_width)?;
    Ok(writer.into_bytes())
}
