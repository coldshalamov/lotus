//! Deterministic codec metrics and reference comparator implementations.
//!
//! Size evidence uses exact interval aggregation. It never substitutes sparse
//! samples for claims about complete integer domains.

use crate::{
    BitReader, BitWriter, LotusError, RECOMMENDED_PROFILES, lotus_encoded_bit_len,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct Workload {
    pub name: &'static str,
    pub values: Vec<u64>,
}

#[derive(Debug, Clone, Copy)]
pub struct UniformDomain {
    pub name: &'static str,
    pub start: u64,
    pub end: u64,
}

impl UniformDomain {
    pub fn values(self) -> u128 {
        self.end as u128 - self.start as u128 + 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComparisonCounts {
    pub wins: u128,
    pub ties: u128,
    pub losses: u128,
}

#[derive(Debug, Clone)]
pub struct LotusConfigResult {
    pub label: &'static str,
    pub j: usize,
    pub d: usize,
    pub total_bits: Option<u128>,
    pub versus_leb128: Option<ComparisonCounts>,
}

#[derive(Debug, Clone)]
pub struct SizeSummary {
    pub workload: &'static str,
    pub start: u64,
    pub end: u64,
    pub values: u128,
    pub lotus: Vec<LotusConfigResult>,
    pub leb128_total_bits: u128,
    pub vlq_total_bits: u128,
    pub elias_gamma_total_bits: u128,
    pub elias_delta_total_bits: u128,
}

pub fn standard_domains() -> &'static [UniformDomain] {
    &[
        UniformDomain {
            name: "tiny",
            start: 0,
            end: 125,
        },
        UniformDomain {
            name: "small_u8",
            start: 0,
            end: u8::MAX as u64,
        },
        UniformDomain {
            name: "medium_1m",
            start: 0,
            end: 1_000_000,
        },
        UniformDomain {
            name: "uniform_u32",
            start: 0,
            end: u32::MAX as u64,
        },
        UniformDomain {
            name: "uniform_u64",
            start: 0,
            end: u64::MAX,
        },
    ]
}

pub fn standard_workloads() -> Vec<Workload> {
    let mut large32 = Vec::with_capacity(4096);
    let mut x32 = 0x9e37_79b9u32;
    for _ in 0..4096 {
        x32 = x32.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        large32.push(u64::from(x32));
    }

    let mut large64 = Vec::with_capacity(4096);
    let mut x64 = 0x9e37_79b9_7f4a_7c15u64;
    for _ in 0..4096 {
        x64 = x64
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        large64.push(x64);
    }

    vec![
        Workload {
            name: "tiny",
            values: (0u64..=125).collect(),
        },
        Workload {
            name: "small",
            values: (0u64..=255).collect(),
        },
        Workload {
            name: "medium",
            values: (0u64..=1_000_000).step_by(1_000).collect(),
        },
        Workload {
            name: "large32",
            values: large32,
        },
        Workload {
            name: "large64",
            values: large64,
        },
    ]
}

fn transition_points(domain: UniformDomain) -> Vec<u128> {
    let start = domain.start as u128;
    let end_exclusive = domain.end as u128 + 1;
    let mut points = BTreeSet::new();
    points.insert(start);
    points.insert(end_exclusive);

    for exponent in 1..=64u32 {
        let power = 1u128 << exponent;
        for point in [power.saturating_sub(2), power.saturating_sub(1)] {
            if point > start && point < end_exclusive {
                points.insert(point);
            }
        }
        if exponent % 7 == 0 && power > start && power < end_exclusive {
            points.insert(power);
        }
    }

    points.into_iter().collect()
}

#[derive(Debug)]
struct LotusAccumulator {
    total_bits: u128,
    comparison: ComparisonCounts,
    valid: bool,
}

pub fn summarize_uniform_domain(domain: UniformDomain) -> SizeSummary {
    let mut lotus = RECOMMENDED_PROFILES
        .iter()
        .map(|_| LotusAccumulator {
            total_bits: 0,
            comparison: ComparisonCounts {
                wins: 0,
                ties: 0,
                losses: 0,
            },
            valid: true,
        })
        .collect::<Vec<_>>();

    let mut leb128_total_bits = 0u128;
    let mut vlq_total_bits = 0u128;
    let mut elias_gamma_total_bits = 0u128;
    let mut elias_delta_total_bits = 0u128;

    let points = transition_points(domain);
    for pair in points.windows(2) {
        let interval_start = pair[0];
        let interval_end = pair[1];
        let count = interval_end - interval_start;
        let value = u64::try_from(interval_start).expect("domain interval fits u64");

        let leb_bits = leb128_bits(value) as u128;
        leb128_total_bits += count * leb_bits;
        vlq_total_bits += count * vlq_bits(value) as u128;
        elias_gamma_total_bits += count * elias_gamma_bits(value) as u128;
        elias_delta_total_bits += count * elias_delta_len(value) as u128;

        for (profile, acc) in RECOMMENDED_PROFILES.iter().zip(&mut lotus) {
            if !acc.valid {
                continue;
            }
            match lotus_encoded_bit_len(
                value,
                profile.config.jumpstarter_bits,
                profile.config.tiers,
            ) {
                Ok(bits) => {
                    let bits = bits as u128;
                    acc.total_bits += count * bits;
                    if bits < leb_bits {
                        acc.comparison.wins += count;
                    } else if bits == leb_bits {
                        acc.comparison.ties += count;
                    } else {
                        acc.comparison.losses += count;
                    }
                }
                Err(_) => acc.valid = false,
            }
        }
    }

    let lotus = RECOMMENDED_PROFILES
        .iter()
        .zip(lotus)
        .map(|(profile, acc)| LotusConfigResult {
            label: profile.label,
            j: profile.config.jumpstarter_bits,
            d: profile.config.tiers,
            total_bits: acc.valid.then_some(acc.total_bits),
            versus_leb128: acc.valid.then_some(acc.comparison),
        })
        .collect();

    SizeSummary {
        workload: domain.name,
        start: domain.start,
        end: domain.end,
        values: domain.values(),
        lotus,
        leb128_total_bits,
        vlq_total_bits,
        elias_gamma_total_bits,
        elias_delta_total_bits,
    }
}

pub fn summarize_uniform_domains(domains: &[UniformDomain]) -> Vec<SizeSummary> {
    domains
        .iter()
        .copied()
        .map(summarize_uniform_domain)
        .collect()
}

pub fn summarize_standard_domains() -> Vec<SizeSummary> {
    summarize_uniform_domains(standard_domains())
}

pub fn leb128_encode(mut value: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(10);
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            return out;
        }
    }
}

pub fn leb128_decode(bytes: &[u8]) -> Result<(u64, usize), LotusError> {
    let mut value = 0u128;
    let mut shift = 0u32;
    for (index, &byte) in bytes.iter().enumerate() {
        if shift >= 70 {
            return Err(LotusError::ValueTooLarge);
        }
        value |= u128::from(byte & 0x7f) << shift;
        if value > u64::MAX as u128 {
            return Err(LotusError::ValueTooLarge);
        }
        if byte & 0x80 == 0 {
            return Ok((value as u64, index + 1));
        }
        shift += 7;
    }
    Err(LotusError::UnexpectedEof)
}

pub fn leb128_bits(value: u64) -> usize {
    let bit_len = (u64::BITS - value.leading_zeros()) as usize;
    bit_len.div_ceil(7).max(1) * 8
}

pub fn vlq_encode(mut value: u64) -> Vec<u8> {
    let mut groups = Vec::with_capacity(10);
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

pub fn vlq_decode(bytes: &[u8]) -> Result<(u64, usize), LotusError> {
    let mut value = 0u128;
    for (index, &byte) in bytes.iter().enumerate() {
        value = (value << 7) | u128::from(byte & 0x7f);
        if value > u64::MAX as u128 {
            return Err(LotusError::ValueTooLarge);
        }
        if byte & 0x80 == 0 {
            return Ok((value as u64, index + 1));
        }
        if index >= 9 {
            return Err(LotusError::ValueTooLarge);
        }
    }
    Err(LotusError::UnexpectedEof)
}

pub fn vlq_bits(value: u64) -> usize {
    leb128_bits(value)
}

fn write_zeroes(writer: &mut BitWriter, mut count: usize) {
    while count > 0 {
        let chunk = count.min(u64::BITS as usize);
        writer
            .write_bits(0, chunk)
            .expect("zero chunk always fits BitWriter");
        count -= chunk;
    }
}

fn write_u128_bits(writer: &mut BitWriter, value: u128, width: usize) {
    if width <= u64::BITS as usize {
        writer
            .write_bits(value as u64, width)
            .expect("low-width u128 value fits");
    } else {
        let high_width = width - u64::BITS as usize;
        writer
            .write_bits((value >> u64::BITS) as u64, high_width)
            .expect("high u128 chunk fits");
        writer
            .write_bits(value as u64, u64::BITS as usize)
            .expect("low u128 chunk fits");
    }
}

pub fn elias_gamma_encode(value: u64) -> Vec<u8> {
    let x = value as u128 + 1;
    let bit_len = (u128::BITS - x.leading_zeros()) as usize;
    let mut writer = BitWriter::with_capacity_bits(2 * bit_len - 1);
    write_zeroes(&mut writer, bit_len - 1);
    write_u128_bits(&mut writer, x, bit_len);
    writer.into_bytes()
}

pub fn elias_gamma_decode(bytes: &[u8]) -> Result<(u64, usize), LotusError> {
    let mut reader = BitReader::new(bytes);
    let mut zeroes = 0usize;
    loop {
        if reader.read_bits(1)? == 1 {
            break;
        }
        zeroes += 1;
        if zeroes > u64::BITS as usize {
            return Err(LotusError::ValueTooLarge);
        }
    }
    let lower = if zeroes == 0 {
        0
    } else {
        reader.read_bits(zeroes)? as u128
    };
    let x = (1u128 << zeroes) | lower;
    let value = x.checked_sub(1).ok_or(LotusError::InvalidEncoding)?;
    if value > u64::MAX as u128 {
        return Err(LotusError::ValueTooLarge);
    }
    Ok((value as u64, reader.bits_consumed()))
}

pub fn elias_gamma_bits(value: u64) -> usize {
    let x = value as u128 + 1;
    let bit_len = (u128::BITS - x.leading_zeros()) as usize;
    2 * bit_len - 1
}

fn write_gamma_positive(writer: &mut BitWriter, value: u64) {
    debug_assert!(value >= 1);
    let bit_len = (u64::BITS - value.leading_zeros()) as usize;
    write_zeroes(writer, bit_len - 1);
    writer
        .write_bits(value, bit_len)
        .expect("positive gamma value fits");
}

fn read_gamma_positive(reader: &mut BitReader<'_>) -> Result<u64, LotusError> {
    let mut zeroes = 0usize;
    loop {
        if reader.read_bits(1)? == 1 {
            break;
        }
        zeroes += 1;
        if zeroes >= u64::BITS as usize {
            return Err(LotusError::ValueTooLarge);
        }
    }
    let lower = if zeroes == 0 {
        0
    } else {
        reader.read_bits(zeroes)?
    };
    Ok((1u64 << zeroes) | lower)
}

pub fn elias_delta_encode(value: u64) -> Vec<u8> {
    let x = value as u128 + 1;
    let payload_bits = (u128::BITS - x.leading_zeros()) as usize;
    let n = payload_bits - 1;
    let np1 = (n + 1) as u64;
    let prefix_bits = 2 * ((u64::BITS - np1.leading_zeros()) as usize) - 1;
    let mut writer = BitWriter::with_capacity_bits(prefix_bits + n);
    write_gamma_positive(&mut writer, np1);
    if n != 0 {
        let suffix = if n == u64::BITS as usize {
            x as u64
        } else {
            (x as u64) & ((1u64 << n) - 1)
        };
        writer
            .write_bits(suffix, n)
            .expect("delta suffix fits");
    }
    writer.into_bytes()
}

pub fn elias_delta_decode(bytes: &[u8]) -> Result<(u64, usize), LotusError> {
    let mut reader = BitReader::new(bytes);
    let np1 = read_gamma_positive(&mut reader)?;
    if np1 == 0 || np1 > 65 {
        return Err(LotusError::InvalidEncoding);
    }
    let n = (np1 - 1) as usize;
    let lower = if n == 0 {
        0
    } else {
        reader.read_bits(n)? as u128
    };
    let x = (1u128 << n) | lower;
    let value = x.checked_sub(1).ok_or(LotusError::InvalidEncoding)?;
    if value > u64::MAX as u128 {
        return Err(LotusError::ValueTooLarge);
    }
    Ok((value as u64, reader.bits_consumed()))
}

pub fn elias_delta_len(value: u64) -> usize {
    let x = value as u128 + 1;
    let n = (u128::BITS - 1 - x.leading_zeros()) as usize;
    let np1 = n + 1;
    let prefix_log = (usize::BITS - 1 - np1.leading_zeros()) as usize;
    2 * prefix_log + 1 + n
}
