use clap::{Parser, Subcommand};
use lotus::{LOTUS_J2D1, LotusError, lotus_decode_u64, lotus_encode_u64};
use std::io::{self, Read};
use std::time::Instant;

#[derive(Parser)]
#[command(author, version, about = "Lotus integer codec CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Encode integers read from stdin (one per line)
    Encode {
        #[arg(short, long, default_value_t = 2)]
        jumpstarter: usize,
        #[arg(short, long, default_value_t = 1)]
        tiers: usize,
    },
    /// Decode a hex-encoded Lotus payload from stdin
    Decode {
        #[arg(short, long, default_value_t = 2)]
        jumpstarter: usize,
        #[arg(short, long, default_value_t = 1)]
        tiers: usize,
    },
    /// Run a micro-benchmark against LEB128 and Elias Delta
    Benchmark {},
}

fn read_stdin_to_string() -> io::Result<String> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    Ok(input)
}

fn encode_mode(j: usize, d: usize) -> Result<(), LotusError> {
    let input = read_stdin_to_string().map_err(|_| LotusError::UnexpectedEof)?;
    for line in input.lines() {
        let value: u64 = line
            .trim()
            .parse()
            .map_err(|_| LotusError::InvalidEncoding)?;
        let encoded = lotus_encode_u64(value, j, d)?;
        println!("{}", hex::encode(encoded));
    }
    Ok(())
}

fn decode_mode(j: usize, d: usize) -> Result<(), LotusError> {
    let input = read_stdin_to_string().map_err(|_| LotusError::UnexpectedEof)?;
    for line in input.lines() {
        let bytes = hex::decode(line.trim()).map_err(|_| LotusError::InvalidEncoding)?;
        let (value, _bits) = lotus_decode_u64(&bytes, j, d)?;
        println!("{}", value);
    }
    Ok(())
}

fn leb128_encode(mut value: u64) -> Vec<u8> {
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

fn elias_delta_len(value: u64) -> usize {
    let mut len = 1;
    let mut bits = 64 - value.leading_zeros() as usize;
    while bits > 1 {
        len += bits;
        bits = 64 - (bits as u64).leading_zeros() as usize;
    }
    len
}

fn run_benchmark() -> Result<(), LotusError> {
    let workloads = vec![
        ("small", (0u64..=255).collect::<Vec<_>>()),
        (
            "medium",
            (0u64..=1_000_000).step_by(10_000).collect::<Vec<_>>(),
        ),
        (
            "large32",
            (0u64..=4_000_000_000)
                .step_by(25_000_000)
                .collect::<Vec<_>>(),
        ),
    ];
    println!("workload,lotus(bits/value),leb128(bytes/value),elias_delta(bits/value)");
    for (name, values) in workloads {
        let start = Instant::now();
        let lotus_bits: usize = values
            .iter()
            .map(|v| {
                lotus_encode_u64(*v, LOTUS_J2D1.0, LOTUS_J2D1.1)
                    .unwrap()
                    .len()
                    * 8
            })
            .sum();
        let lotus_elapsed = start.elapsed();
        let leb_bytes: usize = values.iter().map(|v| leb128_encode(*v).len()).sum();
        let elias_bits: usize = values.iter().map(|v| elias_delta_len(*v)).sum();
        let n = values.len();
        println!(
            "{name},{:.2},{:.2},{:.2} ({:?} encode)",
            lotus_bits as f64 / n as f64,
            leb_bytes as f64 / n as f64,
            elias_bits as f64 / n as f64,
            lotus_elapsed
        );
    }
    Ok(())
}

fn main() -> Result<(), LotusError> {
    let cli = Cli::parse();
    match cli.command {
        Command::Encode { jumpstarter, tiers } => encode_mode(jumpstarter, tiers),
        Command::Decode { jumpstarter, tiers } => decode_mode(jumpstarter, tiers),
        Command::Benchmark {} => run_benchmark(),
    }
}
