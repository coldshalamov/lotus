use clap::{Parser, Subcommand, ValueEnum};
use lotus::metrics::{SizeSummary, demo_lotus_configs, standard_workloads, summarize_sizes};
use lotus::{LotusError, lotus_decode_u64, lotus_encode_u64_framed};
use serde::Serialize;
use std::fs;
use std::io::{self, Read};

#[derive(Parser)]
#[command(author, version, about = "Lotus integer codec CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Csv,
    Json,
    Markdown,
}

#[derive(Subcommand)]
enum Command {
    /// Encode integers read from stdin (one decimal u64 per line)
    Encode {
        #[arg(short, long, default_value_t = 2)]
        jumpstarter: usize,
        #[arg(short, long, default_value_t = 1)]
        tiers: usize,
        /// Prefix each output line with consumed bit length
        #[arg(long)]
        with_bits: bool,
    },
    /// Decode hex-encoded Lotus payloads from stdin (one payload per line)
    Decode {
        #[arg(short, long, default_value_t = 2)]
        jumpstarter: usize,
        #[arg(short, long, default_value_t = 1)]
        tiers: usize,
        /// Print consumed bit length after each value
        #[arg(long)]
        with_bits: bool,
    },
    /// Compute deterministic size summaries used in docs/RESULTS.md
    Benchmark {
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
        /// Write output to a file (prints to stdout when omitted)
        #[arg(long)]
        output: Option<String>,
    },
}

#[derive(Debug, Serialize)]
struct LotusConfigJson {
    label: &'static str,
    j: usize,
    d: usize,
    bits: Option<f64>,
}

#[derive(Debug, Serialize)]
struct SerializableSummary<'a> {
    workload: &'a str,
    lotus: Vec<LotusConfigJson>,
    leb128_bits_per_value: f64,
    vlq_bits_per_value: f64,
    elias_gamma_bits_per_value: f64,
    elias_delta_bits_per_value: f64,
}

fn lotus_configs_json(summary: &SizeSummary) -> Vec<LotusConfigJson> {
    summary
        .lotus
        .iter()
        .map(|c| LotusConfigJson {
            label: c.label,
            j: c.j,
            d: c.d,
            bits: c.bits,
        })
        .collect()
}

fn read_stdin_to_string() -> io::Result<String> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    Ok(input)
}

fn parse_u64_line(line: &str) -> Result<u64, LotusError> {
    line.trim().parse().map_err(|_| LotusError::InvalidEncoding)
}

fn encode_mode(j: usize, d: usize, with_bits: bool) -> Result<(), LotusError> {
    let input = read_stdin_to_string().map_err(|_| LotusError::UnexpectedEof)?;
    for line in input.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let value = parse_u64_line(line)?;
        let encoded = lotus_encode_u64_framed(value, j, d)?;
        if with_bits {
            println!("{} {}", encoded.bit_len, hex::encode(encoded.bytes));
        } else {
            println!("{}", hex::encode(encoded.bytes));
        }
    }
    Ok(())
}

fn decode_mode(j: usize, d: usize, with_bits: bool) -> Result<(), LotusError> {
    let input = read_stdin_to_string().map_err(|_| LotusError::UnexpectedEof)?;
    for line in input.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let bytes = hex::decode(line).map_err(|_| LotusError::InvalidEncoding)?;
        let (value, bits) = lotus_decode_u64(&bytes, j, d)?;
        if with_bits {
            println!("{} {}", value, bits);
        } else {
            println!("{}", value);
        }
    }
    Ok(())
}

fn as_json_rows(summaries: &[SizeSummary]) -> Vec<SerializableSummary<'_>> {
    summaries
        .iter()
        .map(|s| SerializableSummary {
            workload: s.workload,
            lotus: lotus_configs_json(s),
            leb128_bits_per_value: s.leb128_bits,
            vlq_bits_per_value: s.vlq_bits,
            elias_gamma_bits_per_value: s.elias_gamma_bits,
            elias_delta_bits_per_value: s.elias_delta_bits,
        })
        .collect()
}

fn render_csv(summaries: &[SizeSummary]) -> String {
    let mut header = String::from("workload");
    for &(label, _j, _d) in demo_lotus_configs() {
        header.push_str(&format!(",lotus_{label}_bits_per_value"));
    }
    header.push_str(",leb128_bits_per_value,vlq_bits_per_value,elias_gamma_bits_per_value,elias_delta_bits_per_value\n");

    let mut out = header;
    for s in summaries {
        out.push_str(s.workload);
        for c in &s.lotus {
            out.push(',');
            out.push_str(
                &c.bits
                    .map(|v| format!("{v:.4}"))
                    .unwrap_or_else(|| "NA".to_string()),
            );
        }
        out.push_str(&format!(
            ",{:.4},{:.4},{:.4},{:.4}\n",
            s.leb128_bits, s.vlq_bits, s.elias_gamma_bits, s.elias_delta_bits
        ));
    }
    out
}

fn render_markdown(summaries: &[SizeSummary]) -> String {
    let mut out = String::new();
    out.push_str("# Benchmark results\n\n");
    out.push_str("This file is generated by `scripts/reproduce_paper.sh`, which invokes `cargo run --features cli --bin lotus -- benchmark --format markdown --output docs/RESULTS.md`.\n\n");
    out.push_str("Numbers are deterministic size statistics for repository workloads (not runtime throughput).\n\n");
    out.push_str("| workload |");
    for &(label, _j, _d) in demo_lotus_configs() {
        out.push_str(&format!(" lotus {label} (bits/value) |"));
    }
    out.push_str(" LEB128 (bits/value) | VLQ (bits/value) | Elias γ (bits/value) | Elias δ (bits/value) |\n|---|");
    for _ in demo_lotus_configs() {
        out.push_str("---:|");
    }
    out.push_str("---:|---:|---:|---:|\n");
    for s in summaries {
        out.push_str(&format!("| {} ", s.workload));
        for c in &s.lotus {
            out.push_str(&format!(
                "| {} ",
                c.bits
                    .map(|v| format!("{v:.4}"))
                    .unwrap_or_else(|| "NA (out of range)".to_string())
            ));
        }
        out.push_str(&format!(
            "| {:.4} | {:.4} | {:.4} | {:.4} |\n",
            s.leb128_bits, s.vlq_bits, s.elias_gamma_bits, s.elias_delta_bits
        ));
    }
    out
}

fn run_benchmark(format: OutputFormat, output: Option<String>) -> Result<(), LotusError> {
    let summaries = summarize_sizes(&standard_workloads());
    let rendered = match format {
        OutputFormat::Csv => render_csv(&summaries),
        OutputFormat::Json => serde_json::to_string_pretty(&as_json_rows(&summaries))
            .map_err(|_| LotusError::InvalidEncoding)?,
        OutputFormat::Markdown => render_markdown(&summaries),
    };

    if let Some(path) = output {
        fs::write(path, rendered).map_err(|_| LotusError::UnexpectedEof)?;
    } else {
        println!("{rendered}");
    }
    Ok(())
}

fn main() -> Result<(), LotusError> {
    let cli = Cli::parse();
    match cli.command {
        Command::Encode {
            jumpstarter,
            tiers,
            with_bits,
        } => encode_mode(jumpstarter, tiers, with_bits),
        Command::Decode {
            jumpstarter,
            tiers,
            with_bits,
        } => decode_mode(jumpstarter, tiers, with_bits),
        Command::Benchmark { format, output } => run_benchmark(format, output),
    }
}
