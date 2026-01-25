#![forbid(unsafe_code)]

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
}

fn lotus_start(width: usize) -> u64 {
    ((1u128 << width) - 2) as u64
}

fn lotus_payload(value: u64) -> (usize, u64) {
    let mut width = 1usize;
    loop {
        let start = lotus_start(width);
        let end = ((1u128 << (width + 1)) - 3) as u64;
        if value <= end {
            return (width, value - start);
        }
        width += 1;
        if width > 64 {
            return (64, value.saturating_sub(lotus_start(64)));
        }
    }
}

pub fn lotus_encode_u64(value: u64, j_bits: usize, tiers: usize) -> Result<Vec<u8>, LotusError> {
    if j_bits == 0 || tiers == 0 {
        return Err(LotusError::InvalidEncoding);
    }
    let (payload_width, payload_bits) = lotus_payload(value);
    let mut tier_chain = Vec::with_capacity(tiers);
    let mut next_value = payload_width as u64;
    for _ in 0..tiers {
        let (width, bits) = lotus_payload(next_value);
        tier_chain.push((width, bits, next_value));
        next_value = width as u64;
    }

    let (first_width, _, _) = tier_chain.last().ok_or(LotusError::InvalidEncoding)?;
    if *first_width > (1usize << j_bits) {
        return Err(LotusError::JumpstarterOverflow);
    }

    let mut writer = BitWriter::new();
    writer.write_bits((*first_width as u64).saturating_sub(1), j_bits)?;

    for (width, bits, _) in tier_chain.iter().rev() {
        writer.write_bits(*bits, *width)?;
    }

    writer.write_bits(payload_bits, payload_width)?;
    Ok(writer.into_bytes())
}

/// Decode an unsigned 64-bit integer previously encoded with Lotus.
pub fn lotus_decode_u64(
    bytes: &[u8],
    j_bits: usize,
    tiers: usize,
) -> Result<(u64, usize), LotusError> {
    if j_bits == 0 || tiers == 0 {
        return Err(LotusError::InvalidEncoding);
    }
    let mut reader = BitReader::new(bytes);
    let mut bits_consumed = 0usize;
    let jumpstarter = reader.read_bits(j_bits)? as usize;
    bits_consumed += j_bits;
    let mut width = jumpstarter + 1;
    if width == 0 || width > 64 {
        return Err(LotusError::InvalidEncoding);
    }

    let mut payload_width = 0usize;
    for tier_idx in 0..tiers {
        let bits = reader.read_bits(width)?;
        bits_consumed += width;
        let value = lotus_start(width).saturating_add(bits) as u64;
        if tier_idx + 1 == tiers {
            payload_width = value as usize;
        } else {
            width = value as usize;
            if width == 0 || width > 64 {
                return Err(LotusError::InvalidEncoding);
            }
        }
    }

    if payload_width == 0 || payload_width > 64 {
        return Err(LotusError::InvalidEncoding);
    }
    let payload_bits = reader.read_bits(payload_width)?;
    bits_consumed += payload_width;
    let value = lotus_start(payload_width).saturating_add(payload_bits);
    Ok((value, bits_consumed))
}

/// Preset configuration: Jumpstarter 2 bits, 1 tier.
pub const LOTUS_J2D1: (usize, usize) = (2, 1);
/// Preset configuration: Jumpstarter 1 bit, 2 tiers.
pub const LOTUS_J1D2: (usize, usize) = (1, 2);
/// Preset configuration: Jumpstarter 3 bits, 1 tier.
pub const LOTUS_J3D1: (usize, usize) = (3, 1);

/// Compute the exact encoded bit length for a single value.
pub fn lotus_encoded_bits(value: u64, j_bits: usize, tiers: usize) -> Result<usize, LotusError> {
    let encoded = lotus_encode_u64(value, j_bits, tiers)?;
    let (_, bits) = lotus_decode_u64(&encoded, j_bits, tiers)?;
    Ok(bits)
}

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
        for &(j_bits, tiers) in &[LOTUS_J1D2, LOTUS_J2D1, LOTUS_J3D1] {
            for value in [0u64, 1, 2, 4_096, 8_192] {
                let encoded = lotus_encode_u64(value, j_bits, tiers).unwrap();
                let (decoded, _) = lotus_decode_u64(&encoded, j_bits, tiers).unwrap();
                assert_eq!(decoded, value);
            }
        }
    }
}
