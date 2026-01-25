use lotus::{LotusError, lotus_encoded_bits};

#[derive(Clone, Copy)]
struct Config {
    name: &'static str,
    j_bits: usize,
    tiers: usize,
}

fn sample_large64(count: usize) -> Vec<u64> {
    let mut values = Vec::with_capacity(count);
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    for _ in 0..count {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        values.push(state);
    }
    values
}

fn average_bits(values: &[u64], cfg: Config) -> Result<(f64, f64), LotusError> {
    let mut total_bits = 0usize;
    let mut supported = 0usize;
    for &value in values {
        match lotus_encoded_bits(value, cfg.j_bits, cfg.tiers) {
            Ok(bits) => {
                total_bits += bits;
                supported += 1;
            }
            Err(LotusError::JumpstarterOverflow) => {}
            Err(err) => return Err(err),
        }
    }
    let coverage = supported as f64 / values.len() as f64 * 100.0;
    let avg = if supported == 0 {
        0.0
    } else {
        total_bits as f64 / supported as f64
    };
    Ok((avg, coverage))
}

fn main() -> Result<(), LotusError> {
    let configs = [
        Config {
            name: "J1D2",
            j_bits: 1,
            tiers: 2,
        },
        Config {
            name: "J2D1",
            j_bits: 2,
            tiers: 1,
        },
        Config {
            name: "J2D2",
            j_bits: 2,
            tiers: 2,
        },
        Config {
            name: "J3D1",
            j_bits: 3,
            tiers: 1,
        },
    ];

    let small: Vec<u64> = (0u64..=255).collect();
    let medium: Vec<u64> = (0u64..=1_000_000).step_by(10_000).collect();
    let large32: Vec<u64> = (0u64..=4_000_000_000).step_by(25_000_000).collect();
    let large64 = sample_large64(20_000);

    let workloads = [
        ("small", small.as_slice()),
        ("medium", medium.as_slice()),
        ("large32", large32.as_slice()),
        ("large64", large64.as_slice()),
    ];

    println!("# Config sweep (average bits/value)");
    println!();
    println!("Coverage shows the percent of values encodable without overflow for each workload.");
    println!();
    println!("| workload | config | avg bits/value | coverage |");
    println!("|---|---|---|---|");
    for (name, values) in workloads {
        for cfg in configs {
            let (avg, coverage) = average_bits(values, cfg)?;
            println!("| {name} | {} | {:.3} | {:.1}% |", cfg.name, avg, coverage);
        }
    }
    Ok(())
}
