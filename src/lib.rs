#![forbid(unsafe_code)]

pub mod metrics;

#[cfg(feature = "bigint")]
use num_bigint::BigUint;
#[cfg(feature = "bigint")]
use num_traits::One;
use thiserror::Error;

/// Errors emitted by Lotus codecs.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LotusError {
    #[error("payload length exceeds jumpstarter capacity")]
    JumpstarterOverflow,
    #[error("insufficient bits in input slice")]
    UnexpectedEof,
    #[error("invalid lotus encoding")]
    InvalidEncoding,
    #[error("value exceeds algorithmic range for this (J,d) configuration")]
    ValueTooLarge,
}

/// Encoded Lotus payload with explicit framing metadata.
///
/// Lotus encodings are bit-oriented: the final byte may include trailing zero padding bits.
/// `bit_len` records the exact number of meaningful bits to read from `bytes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedLotus {
    /// Backing byte buffer containing the encoded bits (MSB-first).
    pub bytes: Vec<u8>,
    /// Number of meaningful bits in `bytes`.
    pub bit_len: usize,
}

/// Streaming bit writer that appends to an owned buffer.
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

    pub fn into_bytes(mut self) -> Vec<u8> {
        if self.pending_bits > 0 {
            self.buffer.push(self.pending << (8 - self.pending_bits));
        }
        self.buffer
    }

    pub fn bits_written(&self) -> usize {
        self.buffer.len() * 8 + self.pending_bits as usize
    }

    pub fn write_bits(&mut self, value: u64, mut width: usize) -> Result<(), LotusError> {
        let mut remaining_value = value;
        while width > 0 {
            let available = 8 - self.pending_bits;
            let take = available.min(width as u8);
            let shift = width as i32 - take as i32;
            let part = if shift >= 0 {
                ((remaining_value >> shift) & ((1 << take) - 1)) as u16
            } else {
                ((remaining_value << (-shift)) & ((1 << take) - 1)) as u16
            };
            let combined = ((self.pending as u16) << take) | part;
            self.pending = (combined & 0xff) as u8;
            self.pending_bits += take;
            width -= take as usize;
            if self.pending_bits == 8 {
                self.buffer.push(self.pending);
                self.pending = 0;
                self.pending_bits = 0;
            }
            if shift >= 0 {
                remaining_value &= (1u64 << shift) - 1;
            } else {
                remaining_value = 0;
            }
        }
        Ok(())
    }
}

/// Streaming bit reader over a byte slice.
#[derive(Clone, Debug)]
pub struct BitReader<'a> {
    bytes: &'a [u8],
    byte_pos: usize,
    pending: u8,
    pending_bits: u8,
}

impl<'a> BitReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            byte_pos: 0,
            pending: 0,
            pending_bits: 0,
        }
    }

    pub fn read_bits(&mut self, mut width: usize) -> Result<u64, LotusError> {
        let mut value = 0u64;
        while width > 0 {
            if self.pending_bits == 0 {
                self.pending = *self
                    .bytes
                    .get(self.byte_pos)
                    .ok_or(LotusError::UnexpectedEof)?;
                self.byte_pos += 1;
                self.pending_bits = 8;
            }
            let take = self.pending_bits.min(width as u8);
            let shift = self.pending_bits - take;
            let mask = ((1 << take) - 1) as u8;
            let part = (self.pending >> shift) & mask;
            self.pending_bits -= take;
            self.pending &= (1 << self.pending_bits) - 1;
            value = (value << take) | part as u64;
            width -= take as usize;
        }
        Ok(value)
    }

    pub fn bits_consumed(&self) -> usize {
        (self.byte_pos * 8).saturating_sub(self.pending_bits as usize)
    }
}

#[cfg(feature = "bigint")]
fn lotus_encode_value_biguint(value: &BigUint) -> Result<(BigUint, usize), LotusError> {
    let m = value + BigUint::one();
    let mut width = 1usize;
    loop {
        let width_plus_one = width.checked_add(1).ok_or(LotusError::ValueTooLarge)?;
        let start = (BigUint::one() << width) - 2u8;
        let end = (BigUint::one() << width_plus_one) - 3u8;
        if m >= start && m <= end {
            let payload = m - start;
            return Ok((payload, width));
        }
        width = width.checked_add(1).ok_or(LotusError::ValueTooLarge)?;
    }
}

fn lotus_width_for_value(value: u128) -> Result<usize, LotusError> {
    let mut width = 1usize;
    loop {
        let width_u32 = u32::try_from(width).map_err(|_| LotusError::ValueTooLarge)?;
        let start = 1u128
            .checked_shl(width_u32)
            .ok_or(LotusError::ValueTooLarge)?
            .saturating_sub(2);
        let end = 1u128
            .checked_shl(width_u32.saturating_add(1))
            .ok_or(LotusError::ValueTooLarge)?
            .saturating_sub(3);
        if value >= start && value <= end {
            return Ok(width);
        }
        width = width.checked_add(1).ok_or(LotusError::ValueTooLarge)?;
    }
}

fn lotus_encode_fixed(value: u128, width: usize) -> Result<u64, LotusError> {
    if width == 0 {
        return Err(LotusError::ValueTooLarge);
    }
    let width_u32 = u32::try_from(width).map_err(|_| LotusError::ValueTooLarge)?;
    let base = 1u128
        .checked_shl(width_u32)
        .ok_or(LotusError::ValueTooLarge)?;
    let start = base.saturating_sub(2);
    let payload_max = base.saturating_sub(1);
    let end = start.saturating_add(payload_max);
    if value < start || value > end {
        return Err(LotusError::ValueTooLarge);
    }
    let encoded = value - start;
    if encoded > u64::MAX as u128 {
        return Err(LotusError::ValueTooLarge);
    }
    let payload = encoded as u64;
    if let Some(limit) = 1u64.checked_shl(width_u32) {
        debug_assert!(payload < limit);
    }
    Ok(payload)
}

fn lotus_decode_fixed(payload: u64, width: usize) -> Result<u128, LotusError> {
    if width == 0 {
        return Err(LotusError::ValueTooLarge);
    }
    let width_u32 = u32::try_from(width).map_err(|_| LotusError::ValueTooLarge)?;
    let base = 1u128
        .checked_shl(width_u32)
        .ok_or(LotusError::ValueTooLarge)?;
    let payload_max = base.saturating_sub(1);
    let payload_u128 = payload as u128;
    if payload_u128 > payload_max {
        return Err(LotusError::InvalidEncoding);
    }
    if let Some(limit) = 1u64.checked_shl(width_u32) {
        debug_assert!(payload < limit);
    }
    let start = base.saturating_sub(2);
    let decoded = start.saturating_add(payload_u128);
    Ok(decoded)
}

#[cfg(feature = "bigint")]
fn lotus_encode_fixed_biguint(value: &BigUint, width: usize) -> Result<BigUint, LotusError> {
    if width == 0 {
        return Err(LotusError::ValueTooLarge);
    }
    let start = (BigUint::one() << width) - 2u8;
    let payload_max = (BigUint::one() << width) - 1u8;
    let end = &start + &payload_max;
    if value < &start || value > &end {
        return Err(LotusError::ValueTooLarge);
    }
    Ok(value - start)
}

fn max_width_for_config(j_bits: usize, tiers: usize) -> u128 {
    let mut max_width = 1u128 << j_bits;
    for _ in 0..tiers {
        let shift = match max_width.checked_add(1).and_then(|v| u32::try_from(v).ok()) {
            Some(value) if value < 128 => value,
            _ => return u128::MAX,
        };
        let base = match 1u128.checked_shl(shift) {
            Some(value) => value,
            None => return u128::MAX,
        };
        max_width = base.saturating_sub(4);
    }
    max_width
}

#[cfg(feature = "bigint")]
fn write_biguint_bits(
    writer: &mut BitWriter,
    value: &BigUint,
    width: usize,
) -> Result<(), LotusError> {
    let bit_len = value.bits() as usize;
    if bit_len > width {
        return Err(LotusError::InvalidEncoding);
    }
    let mut remaining_zeros = width - bit_len;
    while remaining_zeros > 0 {
        let chunk = remaining_zeros.min(8);
        writer.write_bits(0, chunk)?;
        remaining_zeros -= chunk;
    }
    if bit_len == 0 {
        return Ok(());
    }
    let bytes = value.to_bytes_be();
    let leading_bits = bit_len % 8;
    let mut index = 0;
    if leading_bits != 0 {
        let mask = (1u8 << leading_bits) - 1;
        let part = bytes[0] & mask;
        writer.write_bits(part as u64, leading_bits)?;
        index = 1;
    }
    for &byte in &bytes[index..] {
        writer.write_bits(byte as u64, 8)?;
    }
    Ok(())
}

#[cfg(feature = "bigint")]
/// Encode an arbitrary-precision unsigned integer using Lotus tiered headers.
pub fn lotus_encode_biguint(
    value: &BigUint,
    j_bits: usize,
    tiers: usize,
) -> Result<Vec<u8>, LotusError> {
    if !(1..=8).contains(&j_bits) || tiers == 0 {
        return Err(LotusError::InvalidEncoding);
    }

    let (_payload_bits, payload_width) = lotus_encode_value_biguint(value)?;
    let max_width = max_width_for_config(j_bits, tiers);
    if payload_width as u128 > max_width {
        return Err(LotusError::ValueTooLarge);
    }
    let mut tier_chain: Vec<(u64, usize)> = Vec::with_capacity(tiers);
    let mut current_width = payload_width;

    for _ in 0..tiers {
        let tier_width = lotus_width_for_value(current_width as u128)?;
        tier_chain.push((current_width as u64, tier_width));
        current_width = tier_width;
    }

    if current_width == 0 || current_width > (1usize << j_bits) {
        return Err(LotusError::JumpstarterOverflow);
    }
    let jump_val = (current_width - 1) as u64;

    let mut writer = BitWriter::new();
    writer.write_bits(jump_val, j_bits)?;
    for (value, width) in tier_chain.iter().rev() {
        let encoded = lotus_encode_fixed((*value).into(), *width)?;
        if let Some(limit) = 1u64.checked_shl(*width as u32) {
            debug_assert!(encoded < limit);
        }
        writer.write_bits(encoded, *width)?;
    }
    let payload_value = value + BigUint::one();
    let payload_bits = lotus_encode_fixed_biguint(&payload_value, payload_width)?;
    write_biguint_bits(&mut writer, &payload_bits, payload_width)?;
    Ok(writer.into_bytes())
}

/// Encode an unsigned 64-bit integer using Lotus tiered headers.
pub fn lotus_encode_u64(value: u64, j_bits: usize, tiers: usize) -> Result<Vec<u8>, LotusError> {
    Ok(lotus_encode_u64_framed(value, j_bits, tiers)?.bytes)
}

/// Encode an unsigned 64-bit integer and return exact bit length metadata.
pub fn lotus_encode_u64_framed(
    value: u64,
    j_bits: usize,
    tiers: usize,
) -> Result<EncodedLotus, LotusError> {
    if !(1..=8).contains(&j_bits) || tiers == 0 {
        return Err(LotusError::InvalidEncoding);
    }

    let payload_value = (value as u128).saturating_add(1);
    let payload_width = lotus_width_for_value(payload_value)?;
    let max_width = max_width_for_config(j_bits, tiers);
    if payload_width as u128 > max_width {
        return Err(LotusError::ValueTooLarge);
    }
    let mut chain: Vec<(u128, usize)> = vec![(payload_value, payload_width)];
    let mut current_width = payload_width;

    for _ in 0..tiers {
        let tier_width = lotus_width_for_value(current_width as u128)?;
        chain.push((current_width as u128, tier_width));
        current_width = tier_width;
    }

    if current_width == 0 || current_width > (1usize << j_bits) {
        return Err(LotusError::JumpstarterOverflow);
    }
    let jump_val = (current_width - 1) as u64;

    let mut writer = BitWriter::new();
    writer.write_bits(jump_val, j_bits)?;
    for (value, width) in chain.iter().rev() {
        let encoded = lotus_encode_fixed(*value, *width)?;
        if let Some(limit) = 1u64.checked_shl(*width as u32) {
            debug_assert!(encoded < limit);
        }
        writer.write_bits(encoded, *width)?;
    }
    let bit_len = writer.bits_written();
    Ok(EncodedLotus {
        bytes: writer.into_bytes(),
        bit_len,
    })
}

/// Decode an unsigned 64-bit integer previously encoded with Lotus.
pub fn lotus_decode_u64(
    bytes: &[u8],
    j_bits: usize,
    tiers: usize,
) -> Result<(u64, usize), LotusError> {
    if !(1..=8).contains(&j_bits) || tiers == 0 {
        return Err(LotusError::InvalidEncoding);
    }
    let max_width = max_width_for_config(j_bits, tiers);
    let mut reader = BitReader::new(bytes);
    let start_bits = reader.bits_consumed();
    let jump_val = reader.read_bits(j_bits)? as usize;
    let mut next_width = jump_val + 1;
    if next_width as u128 > max_width {
        return Err(LotusError::ValueTooLarge);
    }

    for _ in 0..tiers {
        let tier_payload = reader.read_bits(next_width)?;
        let width_value = lotus_decode_fixed(tier_payload, next_width)?;
        if width_value == 0 || width_value > max_width {
            return Err(LotusError::ValueTooLarge);
        }
        next_width = usize::try_from(width_value).map_err(|_| LotusError::ValueTooLarge)?;
    }

    let payload = reader.read_bits(next_width)?;
    let m = lotus_decode_fixed(payload, next_width)?;
    if m == 0 {
        return Err(LotusError::InvalidEncoding);
    }
    let value = m - 1;
    if value > u64::MAX as u128 {
        return Err(LotusError::ValueTooLarge);
    }
    let total_bits = reader.bits_consumed().saturating_sub(start_bits);
    Ok((value as u64, total_bits))
}

/// Compute the exact bit length of `lotus_encode_u64(value, j_bits, tiers)`
/// without performing the encoding. Pure arithmetic — no allocations.
pub fn lotus_encoded_bit_len(value: u64, j_bits: usize, tiers: usize) -> Result<usize, LotusError> {
    if !(1..=8).contains(&j_bits) || tiers == 0 {
        return Err(LotusError::InvalidEncoding);
    }

    let payload_value = (value as u128).saturating_add(1);
    let payload_width = lotus_width_for_value(payload_value)?;
    let max_width = max_width_for_config(j_bits, tiers);
    if payload_width as u128 > max_width {
        return Err(LotusError::ValueTooLarge);
    }

    let mut total_tier_width = 0usize;
    let mut current_width = payload_width;
    for _ in 0..tiers {
        let tier_width = lotus_width_for_value(current_width as u128)?;
        total_tier_width = total_tier_width
            .checked_add(tier_width)
            .ok_or(LotusError::ValueTooLarge)?;
        current_width = tier_width;
    }

    if current_width == 0 || current_width > (1usize << j_bits) {
        return Err(LotusError::JumpstarterOverflow);
    }

    let total = j_bits
        .checked_add(total_tier_width)
        .and_then(|v| v.checked_add(payload_width))
        .ok_or(LotusError::ValueTooLarge)?;
    Ok(total)
}

/// Encode `value` into an existing `BitWriter`. Returns bits written.
/// Zero allocation aside from any growth the writer's internal buffer needs.
pub fn lotus_encode_into_writer(
    value: u64,
    j_bits: usize,
    tiers: usize,
    writer: &mut BitWriter,
) -> Result<usize, LotusError> {
    if !(1..=8).contains(&j_bits) || tiers == 0 {
        return Err(LotusError::InvalidEncoding);
    }

    let bits_before = writer.bits_written();

    let payload_value = (value as u128).saturating_add(1);
    let payload_width = lotus_width_for_value(payload_value)?;
    let max_width = max_width_for_config(j_bits, tiers);
    if payload_width as u128 > max_width {
        return Err(LotusError::ValueTooLarge);
    }
    let mut chain: Vec<(u128, usize)> = vec![(payload_value, payload_width)];
    let mut current_width = payload_width;

    for _ in 0..tiers {
        let tier_width = lotus_width_for_value(current_width as u128)?;
        chain.push((current_width as u128, tier_width));
        current_width = tier_width;
    }

    if current_width == 0 || current_width > (1usize << j_bits) {
        return Err(LotusError::JumpstarterOverflow);
    }
    let jump_val = (current_width - 1) as u64;

    writer.write_bits(jump_val, j_bits)?;
    for (value, width) in chain.iter().rev() {
        let encoded = lotus_encode_fixed(*value, *width)?;
        if let Some(limit) = 1u64.checked_shl(*width as u32) {
            debug_assert!(encoded < limit);
        }
        writer.write_bits(encoded, *width)?;
    }

    Ok(writer.bits_written() - bits_before)
}

/// Decode a value from an existing `BitReader`. Returns `(value, bits_consumed)`.
pub fn lotus_decode_from_reader(
    reader: &mut BitReader<'_>,
    j_bits: usize,
    tiers: usize,
) -> Result<(u64, usize), LotusError> {
    if !(1..=8).contains(&j_bits) || tiers == 0 {
        return Err(LotusError::InvalidEncoding);
    }
    let max_width = max_width_for_config(j_bits, tiers);
    let start_bits = reader.bits_consumed();
    let jump_val = reader.read_bits(j_bits)? as usize;
    let mut next_width = jump_val + 1;
    if next_width as u128 > max_width {
        return Err(LotusError::ValueTooLarge);
    }

    for _ in 0..tiers {
        let tier_payload = reader.read_bits(next_width)?;
        let width_value = lotus_decode_fixed(tier_payload, next_width)?;
        if width_value == 0 || width_value > max_width {
            return Err(LotusError::ValueTooLarge);
        }
        next_width = usize::try_from(width_value).map_err(|_| LotusError::ValueTooLarge)?;
    }

    let payload = reader.read_bits(next_width)?;
    let m = lotus_decode_fixed(payload, next_width)?;
    if m == 0 {
        return Err(LotusError::InvalidEncoding);
    }
    let value = m - 1;
    if value > u64::MAX as u128 {
        return Err(LotusError::ValueTooLarge);
    }
    let total_bits = reader.bits_consumed().saturating_sub(start_bits);
    Ok((value as u64, total_bits))
}

/// Preset configuration: Jumpstarter 2 bits, 1 tier.
pub const LOTUS_J2D1: (usize, usize) = (2, 1);
/// Preset configuration: Jumpstarter 1 bit, 2 tiers.
pub const LOTUS_J1D2: (usize, usize) = (1, 2);
/// Preset configuration: Jumpstarter 3 bits, 1 tier.
pub const LOTUS_J3D1: (usize, usize) = (3, 1);

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn round_trip_proptest(value in 0u32..=10_000) {
            let (j_bits, tiers) = LOTUS_J3D1;
            let encoded = lotus_encode_u64(value as u64, j_bits, tiers).unwrap();
            let (decoded, _) = lotus_decode_u64(&encoded, j_bits, tiers).unwrap();
            prop_assert_eq!(decoded, value as u64);
        }
    }

    #[test]
    fn edge_cases() {
        for value in [0u64, 1, 2, 4_096, 8_192] {
            let (j_bits, tiers) = LOTUS_J3D1;
            let encoded = lotus_encode_u64(value, j_bits, tiers).unwrap();
            let (decoded, _) = lotus_decode_u64(&encoded, j_bits, tiers).unwrap();
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn lotus_example_bit_length() {
        let (j_bits, tiers) = (3, 2);
        let encoded = lotus_encode_u64(42, j_bits, tiers).unwrap();
        let (decoded, total_bits) = lotus_decode_u64(&encoded, j_bits, tiers).unwrap();
        assert_eq!(decoded, 42);
        assert_eq!(total_bits, 12);
    }

    #[test]
    fn lotus_j2d1_bit_length() {
        let (j_bits, tiers) = LOTUS_J2D1;
        let encoded = lotus_encode_u64(42, j_bits, tiers).unwrap();
        let (decoded, total_bits) = lotus_decode_u64(&encoded, j_bits, tiers).unwrap();
        assert_eq!(decoded, 42);
        assert_eq!(total_bits, 9);
    }

    #[test]
    fn max_value_round_trip() {
        let (j_bits, tiers) = LOTUS_J3D1;
        let encoded = lotus_encode_u64(u64::MAX, j_bits, tiers).unwrap();
        let (decoded, _) = lotus_decode_u64(&encoded, j_bits, tiers).unwrap();
        assert_eq!(decoded, u64::MAX);
    }

    #[test]
    fn empty_decode_returns_eof() {
        let (j_bits, tiers) = LOTUS_J3D1;
        let err = lotus_decode_u64(&[], j_bits, tiers).unwrap_err();
        assert_eq!(err, LotusError::UnexpectedEof);
    }

    #[test]
    fn framed_api_reports_exact_bits() {
        let framed = lotus_encode_u64_framed(42, LOTUS_J2D1.0, LOTUS_J2D1.1).unwrap();
        assert_eq!(framed.bit_len, 9);
        let (value, consumed) =
            lotus_decode_u64(&framed.bytes, LOTUS_J2D1.0, LOTUS_J2D1.1).unwrap();
        assert_eq!(value, 42);
        assert_eq!(consumed, framed.bit_len);
    }

    #[test]
    fn decode_rejects_truncated_payload() {
        let encoded = lotus_encode_u64(4096, LOTUS_J3D1.0, LOTUS_J3D1.1).unwrap();
        for i in 0..encoded.len() {
            let truncated = &encoded[..i];
            let err = lotus_decode_u64(truncated, LOTUS_J3D1.0, LOTUS_J3D1.1).unwrap_err();
            assert_eq!(err, LotusError::UnexpectedEof);
        }
    }

    #[test]
    fn decode_ignores_trailing_padding_bits() {
        let encoded = lotus_encode_u64(127, LOTUS_J2D1.0, LOTUS_J2D1.1).unwrap();
        let mut extended = encoded.clone();
        extended.push(0xff);
        let (decoded_a, bits_a) = lotus_decode_u64(&encoded, LOTUS_J2D1.0, LOTUS_J2D1.1).unwrap();
        let (decoded_b, bits_b) = lotus_decode_u64(&extended, LOTUS_J2D1.0, LOTUS_J2D1.1).unwrap();
        assert_eq!(decoded_a, decoded_b);
        assert_eq!(bits_a, bits_b);
    }

    #[test]
    fn fixed_width_payloads_are_consecutive() {
        for width in 1..=12 {
            let max_payload = 1u64 << width;
            let mut expected = (1u128 << width).saturating_sub(2);
            for payload in 0..max_payload {
                let decoded = lotus_decode_fixed(payload, width).unwrap();
                assert_eq!(decoded, expected);
                expected += 1;
            }
        }
    }

    #[test]
    fn density_invariant_fixed_width() {
        for width in 1..=16 {
            let max_payload = 1u64 << width;
            let start = (1u128 << width).saturating_sub(2);
            let mut seen = Vec::with_capacity(max_payload as usize);
            for payload in 0..max_payload {
                let decoded = lotus_decode_fixed(payload, width).unwrap();
                seen.push(decoded);
            }
            seen.sort();
            for (idx, value) in seen.into_iter().enumerate() {
                assert_eq!(value, start + idx as u128);
            }
        }
    }

    fn leb128_len_u32(mut value: u32) -> usize {
        let mut bytes = 1;
        while value >= 0x80 {
            value >>= 7;
            bytes += 1;
        }
        bytes
    }

    #[test]
    fn lotus_beats_leb128_for_uniform_u32_samples() {
        let (j_bits, tiers) = LOTUS_J1D2;
        let max_width = max_width_for_config(j_bits, tiers);
        let max_value = (1u128 << (max_width + 1)).saturating_sub(4);
        let mut seed: u64 = 0x1234_5678_9abc_def0;
        let mut lotus_better = 0usize;
        let samples = 10_000usize;
        for _ in 0..samples {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let sample = (seed >> 32) as u32;
            let value = (sample as u128 % (max_value + 1)) as u32;
            let encoded = lotus_encode_u64(value as u64, j_bits, tiers).unwrap();
            let (_, lotus_bits) = lotus_decode_u64(&encoded, j_bits, tiers).unwrap();
            let leb_bits = leb128_len_u32(value) * 8;
            if lotus_bits < leb_bits {
                lotus_better += 1;
            }
        }
        assert!(
            lotus_better > samples / 2,
            "Lotus should beat LEB128 more often than not"
        );
    }

    #[test]
    fn bit_len_matches_framed() {
        for (j, d) in [LOTUS_J3D1, LOTUS_J2D1, LOTUS_J1D2, (3, 2)] {
            for v in [0u64, 1, 5, 42, 255, 65535, 1_000_000] {
                let framed = lotus_encode_u64_framed(v, j, d).unwrap();
                let bit_len = lotus_encoded_bit_len(v, j, d).unwrap();
                assert_eq!(bit_len, framed.bit_len, "value={v} j={j} d={d}");
            }
        }
    }

    #[test]
    fn streaming_writer_concatenates() {
        let (j, d) = (3, 2);
        let mut writer = BitWriter::new();
        let bits_a = lotus_encode_into_writer(42, j, d, &mut writer).unwrap();
        let bits_b = lotus_encode_into_writer(7, j, d, &mut writer).unwrap();
        let bytes = writer.into_bytes();

        let mut reader = BitReader::new(&bytes);
        let (a, used_a) = lotus_decode_from_reader(&mut reader, j, d).unwrap();
        let (b, used_b) = lotus_decode_from_reader(&mut reader, j, d).unwrap();
        assert_eq!(a, 42);
        assert_eq!(b, 7);
        assert_eq!(used_a, bits_a);
        assert_eq!(used_b, bits_b);
    }

    #[test]
    fn streaming_writer_matches_standalone() {
        let (j, d) = (3, 2);
        let value = 12345u64;

        let standalone = lotus_encode_u64(value, j, d).unwrap();
        let standalone_bit_len = lotus_encode_u64_framed(value, j, d).unwrap().bit_len;

        let mut writer = BitWriter::new();
        lotus_encode_into_writer(value, j, d, &mut writer).unwrap();
        let streamed = writer.into_bytes();

        // Bytes should match exactly (both pad zeros at end of last byte).
        assert_eq!(standalone, streamed);

        // Decoding both via standalone API yields same result.
        let (v1, _) = lotus_decode_u64(&standalone, j, d).unwrap();
        let (v2, _) = lotus_decode_u64(&streamed, j, d).unwrap();
        assert_eq!(v1, v2);
        assert_eq!(v1, value);
        let _ = standalone_bit_len;
    }

    #[test]
    fn streaming_reader_alignment() {
        // Encode three values back-to-back, decode them back, verify reader advances correctly.
        let (j, d) = (3, 2);
        let values = [0u64, 100, 65535];

        let mut writer = BitWriter::new();
        let mut expected_bits = 0usize;
        for &v in &values {
            let b = lotus_encode_into_writer(v, j, d, &mut writer).unwrap();
            expected_bits += b;
        }
        let total_bits = writer.bits_written();
        assert_eq!(total_bits, expected_bits);
        let bytes = writer.into_bytes();

        let mut reader = BitReader::new(&bytes);
        for &expected in &values {
            let (v, _) = lotus_decode_from_reader(&mut reader, j, d).unwrap();
            assert_eq!(v, expected);
        }
    }
}
