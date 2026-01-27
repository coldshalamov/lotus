#![forbid(unsafe_code)]

use thiserror::Error;
#[cfg(feature = "bigint")]
use num_bigint::BigUint;
#[cfg(feature = "bigint")]
use num_traits::One;

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

/// Encode a single integer using Lotus unfolding, returning its payload bits and width.
fn lotus_encode_value(value: u64) -> Result<(u64, usize), LotusError> {
    let m = (value as u128) + 1;
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
        if m >= start && m <= end {
            let payload = m - start;
            return Ok((payload as u64, width));
        }
        width += 1;
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

fn lotus_decode_value(payload: u64, width: usize) -> Result<u64, LotusError> {
    if width == 0 {
        return Err(LotusError::ValueTooLarge);
    }
    let width_u32 = u32::try_from(width).map_err(|_| LotusError::ValueTooLarge)?;
    let start = 1u128
        .checked_shl(width_u32)
        .ok_or(LotusError::ValueTooLarge)?
        .saturating_sub(2);
    let m = (payload as u128).saturating_add(start);
    if m == 0 {
        return Err(LotusError::InvalidEncoding);
    }
    let value = m - 1;
    if value > u64::MAX as u128 {
        return Err(LotusError::ValueTooLarge);
    }
    Ok(value as u64)
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

    let (payload_bits, payload_width) = lotus_encode_value_biguint(value)?;
    let max_width = max_width_for_config(j_bits, tiers);
    if payload_width as u128 > max_width {
        return Err(LotusError::ValueTooLarge);
    }
    let mut tier_chain: Vec<(u64, usize)> = Vec::with_capacity(tiers);
    let mut current_width = payload_width;

    for _ in 0..tiers {
        let (tier_bits, tier_width) = lotus_encode_value(current_width as u64)?;
        tier_chain.push((tier_bits, tier_width));
        current_width = tier_width;
    }

    if current_width == 0 || current_width > (1usize << j_bits) {
        return Err(LotusError::JumpstarterOverflow);
    }
    let jump_val = (current_width - 1) as u64;

    let mut writer = BitWriter::new();
    writer.write_bits(jump_val, j_bits)?;
    for (bits, width) in tier_chain.iter().rev() {
        writer.write_bits(*bits, *width)?;
    }
    write_biguint_bits(&mut writer, &payload_bits, payload_width)?;
    Ok(writer.into_bytes())
}

/// Encode an unsigned 64-bit integer using Lotus tiered headers.
pub fn lotus_encode_u64(value: u64, j_bits: usize, tiers: usize) -> Result<Vec<u8>, LotusError> {
    if !(1..=8).contains(&j_bits) || tiers == 0 {
        return Err(LotusError::InvalidEncoding);
    }

    let (payload_bits, payload_width) = lotus_encode_value(value)?;
    let max_width = max_width_for_config(j_bits, tiers);
    if payload_width as u128 > max_width {
        return Err(LotusError::ValueTooLarge);
    }
    let mut chain: Vec<(u64, usize)> = vec![(payload_bits, payload_width)];
    let mut current_width = payload_width;

    for _ in 0..tiers {
        let (tier_bits, tier_width) = lotus_encode_value(current_width as u64)?;
        chain.push((tier_bits, tier_width));
        current_width = tier_width;
    }

    if current_width == 0 || current_width > (1usize << j_bits) {
        return Err(LotusError::JumpstarterOverflow);
    }
    let jump_val = (current_width - 1) as u64;

    let mut writer = BitWriter::new();
    writer.write_bits(jump_val, j_bits)?;
    for (bits, width) in chain.iter().rev() {
        writer.write_bits(*bits, *width)?;
    }
    Ok(writer.into_bytes())
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
        let width_value = lotus_decode_value(tier_payload, next_width)? as usize;
        if width_value == 0 || width_value as u128 > max_width {
            return Err(LotusError::ValueTooLarge);
        }
        next_width = width_value;
    }

    let payload = reader.read_bits(next_width)?;
    let value = lotus_decode_value(payload, next_width)?;
    let total_bits = reader.bits_consumed().saturating_sub(start_bits);
    Ok((value, total_bits))
}

/// Preset configuration: Jumpstarter 2 bits, 1 tier.
pub const LOTUS_J2D1: (usize, usize) = (2, 1);
/// Preset configuration: Jumpstarter 1 bit, 2 tiers.
pub const LOTUS_J1D2: (usize, usize) = (1, 2);
/// Preset configuration: Jumpstarter 3 bits, 1 tier.
pub const LOTUS_J3D1: (usize, usize) = (3, 1);

#[cfg(feature = "small-int-fastpath")]
pub fn lotus_encode_small(value: u64) -> Result<Vec<u8>, LotusError> {
    if value < 128 {
        Ok(vec![value as u8])
    } else {
        lotus_encode_u64(value, LOTUS_J2D1.0, LOTUS_J2D1.1)
    }
}

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
        assert_eq!(total_bits, 13);
    }

    #[test]
    fn lotus_j2d1_bit_length() {
        let (j_bits, tiers) = LOTUS_J2D1;
        let encoded = lotus_encode_u64(42, j_bits, tiers).unwrap();
        let (decoded, total_bits) = lotus_decode_u64(&encoded, j_bits, tiers).unwrap();
        assert_eq!(decoded, 42);
        assert_eq!(total_bits, 10);
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
}
