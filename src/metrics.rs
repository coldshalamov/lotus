use crate::{LOTUS_J2D1, LOTUS_J3D1, lotus_decode_u64, lotus_encode_u64};

#[derive(Debug, Clone)]
pub struct Workload {
    pub name: &'static str,
    pub values: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct SizeSummary {
    pub workload: &'static str,
    pub lotus_j2d1_bits: Option<f64>,
    pub lotus_j3d1_bits: Option<f64>,
    pub leb128_bits: f64,
    pub elias_delta_bits: f64,
}

pub fn standard_workloads() -> Vec<Workload> {
    vec![
        Workload {
            name: "small",
            values: (0u64..=255).collect(),
        },
        Workload {
            name: "medium",
            values: (0u64..=1_000_000).step_by(10_000).collect(),
        },
        Workload {
            name: "large32",
            values: (0u64..=4_000_000_000).step_by(25_000_000).collect(),
        },
    ]
}

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

pub fn elias_delta_len(value: u64) -> usize {
    let n = value.saturating_add(1);
    let n_bits = 64 - n.leading_zeros() as usize;
    let n_bits_bits = usize::BITS as usize - n_bits.leading_zeros() as usize;
    (2 * n_bits_bits - 1) + (n_bits - 1)
}

fn average_lotus_bits(values: &[u64], cfg: (usize, usize)) -> Option<f64> {
    let mut total = 0usize;
    for &v in values {
        let encoded = lotus_encode_u64(v, cfg.0, cfg.1).ok()?;
        let (_, bits) = lotus_decode_u64(&encoded, cfg.0, cfg.1).ok()?;
        total += bits;
    }
    Some(total as f64 / values.len() as f64)
}

pub fn summarize_sizes(workloads: &[Workload]) -> Vec<SizeSummary> {
    workloads
        .iter()
        .map(|w| {
            let n = w.values.len() as f64;
            let leb_total: usize = w.values.iter().map(|&v| leb128_encode(v).len() * 8).sum();
            let elias_total: usize = w.values.iter().map(|&v| elias_delta_len(v)).sum();

            SizeSummary {
                workload: w.name,
                lotus_j2d1_bits: average_lotus_bits(&w.values, LOTUS_J2D1),
                lotus_j3d1_bits: average_lotus_bits(&w.values, LOTUS_J3D1),
                leb128_bits: leb_total as f64 / n,
                elias_delta_bits: elias_total as f64 / n,
            }
        })
        .collect()
}
