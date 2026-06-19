use crate::{LotusError, lotus_decode_u64, lotus_encode_u64};

/// A named integer-encoding workload used for deterministic size statistics.
#[derive(Debug, Clone)]
pub struct Workload {
    pub name: &'static str,
    pub values: Vec<u64>,
}

/// Result of measuring one Lotus `(J, d)` configuration over a workload.
///
/// `bits` is `None` when some workload values fall outside the configuration's
/// representable range.
#[derive(Debug, Clone)]
pub struct LotusConfigResult {
    pub label: &'static str,
    pub j: usize,
    pub d: usize,
    pub bits: Option<f64>,
}

/// Deterministic size statistics for one workload across Lotus configs and
/// reference varint codecs. All figures are **bits per value**.
#[derive(Debug, Clone)]
pub struct SizeSummary {
    pub workload: &'static str,
    pub lotus: Vec<LotusConfigResult>,
    pub leb128_bits: f64,
    pub vlq_bits: f64,
    pub elias_gamma_bits: f64,
    pub elias_delta_bits: f64,
}

/// The Lotus `(J, d)` configurations surfaced in docs and the interactive demo.
///
/// These are the configs whose density trade-offs are most instructive:
/// small/large jumpstarter and single/double tier.
pub fn demo_lotus_configs() -> &'static [(&'static str, usize, usize)] {
    &[
        ("J1D2", 1, 2),
        ("J2D1", 2, 1),
        ("J3D1", 3, 1),
        ("J3D2", 3, 2),
    ]
}

pub fn standard_workloads() -> Vec<Workload> {
    vec![
        Workload {
            name: "small",
            values: (0u64..=255).collect(),
        },
        Workload {
            name: "medium",
            values: (0u64..1_000_000).step_by(10_000).collect(),
        },
        Workload {
            name: "large32",
            values: (0u64..4_000_000_000).step_by(25_000_000).collect(),
        },
    ]
}

// ---------------------------------------------------------------------------
// LEB128 (little-endian base-128 varint)
// ---------------------------------------------------------------------------

/// Encode `value` using LEB128 (little-endian base-128), as used by protobuf /
/// DWARF / WebAssembly.
pub fn leb128_encode(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
            out.push(byte);
        } else {
            out.push(byte);
            break;
        }
    }
    out
}

/// Decode a single LEB128 value from `bytes`.
///
/// Returns `(value, bytes_consumed)`.
pub fn leb128_decode(bytes: &[u8]) -> Result<(u64, usize), LotusError> {
    let mut value = 0u64;
    let mut shift = 0u32;
    for (i, &byte) in bytes.iter().enumerate() {
        if shift >= 64 {
            return Err(LotusError::ValueTooLarge);
        }
        value |= u64::from(byte & 0x7f) << shift;
        let consumed = i + 1;
        if byte & 0x80 == 0 {
            return Ok((value, consumed));
        }
        shift += 7;
    }
    Err(LotusError::UnexpectedEof)
}

/// Bit length of `leb128_encode(value)` without allocating.
pub fn leb128_bits(value: u64) -> usize {
    let bit_len = 64 - value.leading_zeros() as usize;
    let bytes = bit_len.div_ceil(7).max(1);
    bytes * 8
}

// ---------------------------------------------------------------------------
// VLQ (big-endian base-128 varint, MIDI / Bitcoin style)
// ---------------------------------------------------------------------------

/// Encode `value` using big-endian base-128 VLQ. The most significant 7-bit
/// group comes first; every byte except the last has the continuation bit set.
pub fn vlq_encode(mut value: u64) -> Vec<u8> {
    let mut groups: Vec<u8> = Vec::new();
    loop {
        groups.push((value & 0x7f) as u8);
        value >>= 7;
        if value == 0 {
            break;
        }
    }
    groups.reverse();
    let last = groups.len() - 1;
    for byte in &mut groups[..last] {
        *byte |= 0x80;
    }
    groups
}

/// Decode a single big-endian base-128 VLQ value from `bytes`.
///
/// Returns `(value, bytes_consumed)`.
pub fn vlq_decode(bytes: &[u8]) -> Result<(u64, usize), LotusError> {
    let mut value = 0u64;
    for (i, &byte) in bytes.iter().enumerate() {
        if value > (u64::MAX >> 7) {
            return Err(LotusError::ValueTooLarge);
        }
        value = (value << 7) | u64::from(byte & 0x7f);
        let consumed = i + 1;
        if byte & 0x80 == 0 {
            return Ok((value, consumed));
        }
    }
    Err(LotusError::UnexpectedEof)
}

/// Bit length of `vlq_encode(value)` without allocating.
pub fn vlq_bits(value: u64) -> usize {
    leb128_bits(value)
}

// ---------------------------------------------------------------------------
// Elias gamma (bit-oriented universal code)
// ---------------------------------------------------------------------------
//
// Elias gamma encodes the positive integer x as: floor(log2(x)) zero bits,
// followed by the binary representation of x (which begins with a 1).
// Length = 2*floor(log2(x)) + 1.
//
// Lotus's public API maps a stored value `v` (v >= 0) to the coded integer
// `x = v + 1`, mirroring the value+1 convention used inside Lotus itself.
//
// Because `x` must fit u64, the maximum storable value is `u64::MAX - 1`.

/// Encode `value` using Elias gamma. Returns the packed MSB-first bitstream
/// (final byte is zero-padded). Valid for `value <= u64::MAX - 1`.
pub fn elias_gamma_encode(value: u64) -> Vec<u8> {
    use crate::BitWriter;
    let x = value.saturating_add(1);
    let n = 64 - x.leading_zeros() as usize; // bit length of x, n >= 1
    let mut writer = BitWriter::new();
    // floor(log2(x)) = n - 1 leading zero bits.
    let _ = writer.write_bits(0, n - 1);
    let _ = writer.write_bits(x, n);
    writer.into_bytes()
}

/// Decode a single Elias gamma value from `bytes`.
///
/// Returns `(value, bits_consumed)`.
pub fn elias_gamma_decode(bytes: &[u8]) -> Result<(u64, usize), LotusError> {
    use crate::BitReader;
    let mut reader = BitReader::new(bytes);
    let start = reader.bits_consumed();
    let mut zeros = 0usize;
    loop {
        if reader.read_bits(1)? == 1 {
            break;
        }
        zeros += 1;
        if zeros >= 64 {
            return Err(LotusError::InvalidEncoding);
        }
    }
    let rest = if zeros == 0 {
        0
    } else {
        reader.read_bits(zeros)?
    };
    let x = (1u64 << zeros) | rest;
    let value = x.checked_sub(1).ok_or(LotusError::InvalidEncoding)?;
    let bits = reader.bits_consumed().saturating_sub(start);
    Ok((value, bits))
}

/// Bit length of `elias_gamma_encode(value)` without allocating.
pub fn elias_gamma_bits(value: u64) -> usize {
    let x = value.saturating_add(1);
    let n = 64 - x.leading_zeros() as usize; // n >= 1
    // 2 * (n - 1) + 1
    2 * (n - 1) + 1
}

// ---------------------------------------------------------------------------
// Elias delta (bit-oriented universal code)
// ---------------------------------------------------------------------------
//
// Elias delta encodes the positive integer x as: Elias gamma(N + 1), followed
// by the low N bits of x, where N = floor(log2(x)).
// Length = gamma_len(N + 1) + N.
//
// Same `x = value + 1` convention as Elias gamma, so the max storable value
// is `u64::MAX - 1`.

/// Encode `value` using Elias delta. Returns the packed MSB-first bitstream
/// (final byte is zero-padded). Valid for `value <= u64::MAX - 1`.
pub fn elias_delta_encode(value: u64) -> Vec<u8> {
    use crate::BitWriter;
    let x = value.saturating_add(1);
    let n = (64 - x.leading_zeros() as usize) - 1; // N = floor(log2(x)), n >= 0
    let np1 = (n + 1) as u64; // gamma codes N + 1
    let l_bits = 64 - np1.leading_zeros() as usize; // bit length of N + 1, >= 1
    let l = l_bits - 1; // L = floor(log2(N + 1)) leading zero bits

    let mut writer = BitWriter::new();
    let _ = writer.write_bits(0, l);
    let _ = writer.write_bits(np1, l_bits);
    if n > 0 {
        let _ = writer.write_bits(x & ((1u64 << n) - 1), n);
    }
    writer.into_bytes()
}

/// Decode a single Elias delta value from `bytes`.
///
/// Returns `(value, bits_consumed)`.
pub fn elias_delta_decode(bytes: &[u8]) -> Result<(u64, usize), LotusError> {
    use crate::BitReader;
    let mut reader = BitReader::new(bytes);
    let start = reader.bits_consumed();

    // Decode the leading Elias gamma to recover N + 1.
    let mut zeros = 0usize;
    loop {
        if reader.read_bits(1)? == 1 {
            break;
        }
        zeros += 1;
        if zeros >= 64 {
            return Err(LotusError::InvalidEncoding);
        }
    }
    let rest = if zeros == 0 {
        0
    } else {
        reader.read_bits(zeros)?
    };
    let np1 = (1u64 << zeros) | rest; // N + 1
    if np1 == 0 || np1 > 64 {
        return Err(LotusError::InvalidEncoding);
    }
    let big_n = (np1 - 1) as usize; // N = floor(log2(x))

    let lower = if big_n == 0 {
        0
    } else {
        reader.read_bits(big_n)?
    };
    let x = (1u64 << big_n) | lower;
    let value = x.checked_sub(1).ok_or(LotusError::InvalidEncoding)?;
    let bits = reader.bits_consumed().saturating_sub(start);
    Ok((value, bits))
}

/// Bit length of `elias_delta_encode(value)` without allocating.
///
/// This is the existing deterministic length helper retained for backward
/// compatibility; it returns the length **in bits**.
pub fn elias_delta_len(value: u64) -> usize {
    let n = value.saturating_add(1);
    let n_bits = 64 - n.leading_zeros() as usize;
    let n_bits_bits = usize::BITS as usize - n_bits.leading_zeros() as usize;
    (2 * n_bits_bits - 1) + (n_bits - 1)
}

// ---------------------------------------------------------------------------
// Aggregation
// ---------------------------------------------------------------------------

fn average_lotus_bits(values: &[u64], j: usize, d: usize) -> Option<f64> {
    let mut total = 0usize;
    for &v in values {
        let encoded = lotus_encode_u64(v, j, d).ok()?;
        let (_, bits) = lotus_decode_u64(&encoded, j, d).ok()?;
        total += bits;
    }
    Some(total as f64 / values.len() as f64)
}

pub fn summarize_sizes(workloads: &[Workload]) -> Vec<SizeSummary> {
    workloads
        .iter()
        .map(|w| {
            let n = w.values.len().max(1) as f64;
            let leb_total: usize = w.values.iter().map(|&v| leb128_bits(v)).sum();
            let vlq_total: usize = w.values.iter().map(|&v| vlq_bits(v)).sum();
            let gamma_total: usize = w.values.iter().map(|&v| elias_gamma_bits(v)).sum();
            let delta_total: usize = w.values.iter().map(|&v| elias_delta_len(v)).sum();

            let lotus: Vec<LotusConfigResult> = demo_lotus_configs()
                .iter()
                .map(|&(label, j, d)| LotusConfigResult {
                    label,
                    j,
                    d,
                    bits: average_lotus_bits(&w.values, j, d),
                })
                .collect();

            SizeSummary {
                workload: w.name,
                lotus,
                leb128_bits: leb_total as f64 / n,
                vlq_bits: vlq_total as f64 / n,
                elias_gamma_bits: gamma_total as f64 / n,
                elias_delta_bits: delta_total as f64 / n,
            }
        })
        .collect()
}
