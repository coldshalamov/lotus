use clap::{Parser, Subcommand, ValueEnum};
use lotus::metrics::{ComparisonCounts, SizeSummary, summarize_standard_domains};
use lotus::{LOTUS_DENSE_U64, LotusError, lotus_decode_u64, lotus_encode_u64_framed};
use serde::Serialize;
use std::fs;
use std::io::{self, Read};

#[derive(Parser)]
#[command(author, version, about = "Canonical Lotus integer codec CLI")]
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
    /// Encode decimal u64 values read from stdin, one per line.
    Encode {
        #[arg(
            short,
            long,
            default_value_t = LOTUS_DENSE_U64.jumpstarter_bits
        )]
        jumpstarter: usize,
        #[arg(short, long, default_value_t = LOTUS_DENSE_U64.tiers)]
        tiers: usize,
        /// Prefix each output line with its exact meaningful bit length.
        #[arg(long)]
        with_bits: bool,
    },
    /// Decode hex Lotus payloads read from stdin, one per line.
    Decode {
        #[arg(
            short,
            long,
            default_value_t = LOTUS_DENSE_U64.jumpstarter_bits
        )]
        jumpstarter: usize,
        #[arg(short, long, default_value_t = LOTUS_DENSE_U64.tiers)]
        tiers: usize,
        /// Print the exact number of consumed bits after each value.
        #[arg(long)]
        with_bits: bool,
    },
    /// Generate exact deterministic size evidence.
    Benchmark {
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
        /// Write output to a file instead of stdout.
        #[arg(long)]
        output: Option<String>,
    },
}

#[derive(Debug, Serialize)]
struct ComparisonJson {
    wins: String,
    ties: String,
    losses: String,
    win_percent: String,
    tie_percent: String,
    loss_percent: String,
}

#[derive(Debug, Serialize)]
struct LotusConfigJson {
    label: &'static str,
    j: usize,
    d: usize,
    total_bits: Option<String>,
    bits_per_value: Option<String>,
    versus_leb128: Option<ComparisonJson>,
}

#[derive(Debug, Serialize)]
struct SerializableSummary {
    workload: &'static str,
    start: String,
    end: String,
    values: String,
    lotus: Vec<LotusConfigJson>,
    leb128_total_bits: String,
    leb128_bits_per_value: String,
    vlq_total_bits: String,
    vlq_bits_per_value: String,
    elias_gamma_total_bits: String,
    elias_gamma_bits_per_value: String,
    elias_delta_total_bits: String,
    elias_delta_bits_per_value: String,
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
    for line in input.lines().map(str::trim).filter(|line| !line.is_empty()) {
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
    for line in input.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let bytes = hex::decode(line).map_err(|_| LotusError::InvalidEncoding)?;
        let (value, bits) = lotus_decode_u64(&bytes, j, d)?;
        if with_bits {
            println!("{value} {bits}");
        } else {
            println!("{value}");
        }
    }
    Ok(())
}

fn format_ratio(numerator: u128, denominator: u128, decimals: u32) -> String {
    debug_assert!(denominator != 0);
    let scale = 10u128.pow(decimals);
    let scaled = (numerator * scale + denominator / 2) / denominator;
    let whole = scaled / scale;
    let fraction = scaled % scale;
    format!(
        "{whole}.{fraction:0width$}",
        width = usize::try_from(decimals).expect("decimal count fits usize")
    )
}

fn format_percent(count: u128, total: u128) -> String {
    format_ratio(count * 100, total, 6)
}

fn comparison_json(counts: ComparisonCounts, total: u128) -> ComparisonJson {
    ComparisonJson {
        wins: counts.wins.to_string(),
        ties: counts.ties.to_string(),
        losses: counts.losses.to_string(),
        win_percent: format_percent(counts.wins, total),
        tie_percent: format_percent(counts.ties, total),
        loss_percent: format_percent(counts.losses, total),
    }
}

fn as_json_rows(summaries: &[SizeSummary]) -> Vec<SerializableSummary> {
    summaries
        .iter()
        .map(|summary| SerializableSummary {
            workload: summary.workload,
            start: summary.start.to_string(),
            end: summary.end.to_string(),
            values: summary.values.to_string(),
            lotus: summary
                .lotus
                .iter()
                .map(|config| LotusConfigJson {
                    label: config.label,
                    j: config.j,
                    d: config.d,
                    total_bits: config.total_bits.map(|value| value.to_string()),
                    bits_per_value: config
                        .total_bits
                        .map(|value| format_ratio(value, summary.values, 6)),
                    versus_leb128: config
                        .versus_leb128
                        .map(|counts| comparison_json(counts, summary.values)),
                })
                .collect(),
            leb128_total_bits: summary.leb128_total_bits.to_string(),
            leb128_bits_per_value: format_ratio(summary.leb128_total_bits, summary.values, 6),
            vlq_total_bits: summary.vlq_total_bits.to_string(),
            vlq_bits_per_value: format_ratio(summary.vlq_total_bits, summary.values, 6),
            elias_gamma_total_bits: summary.elias_gamma_total_bits.to_string(),
            elias_gamma_bits_per_value: format_ratio(
                summary.elias_gamma_total_bits,
                summary.values,
                6,
            ),
            elias_delta_total_bits: summary.elias_delta_total_bits.to_string(),
            elias_delta_bits_per_value: format_ratio(
                summary.elias_delta_total_bits,
                summary.values,
                6,
            ),
        })
        .collect()
}

fn render_csv(summaries: &[SizeSummary]) -> String {
    let mut out = String::from(
        "workload,start,end,values,profile,j,d,total_bits,bits_per_value,wins,ties,losses,win_percent,tie_percent,loss_percent,leb128_bits_per_value\n",
    );
    for summary in summaries {
        for config in &summary.lotus {
            let total_bits = config
                .total_bits
                .map(|value| value.to_string())
                .unwrap_or_else(|| "NA".to_string());
            let bits_per_value = config
                .total_bits
                .map(|value| format_ratio(value, summary.values, 6))
                .unwrap_or_else(|| "NA".to_string());
            let (wins, ties, losses, win_percent, tie_percent, loss_percent) =
                if let Some(counts) = config.versus_leb128 {
                    (
                        counts.wins.to_string(),
                        counts.ties.to_string(),
                        counts.losses.to_string(),
                        format_percent(counts.wins, summary.values),
                        format_percent(counts.ties, summary.values),
                        format_percent(counts.losses, summary.values),
                    )
                } else {
                    (
                        "NA".to_string(),
                        "NA".to_string(),
                        "NA".to_string(),
                        "NA".to_string(),
                        "NA".to_string(),
                        "NA".to_string(),
                    )
                };
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                summary.workload,
                summary.start,
                summary.end,
                summary.values,
                config.label,
                config.j,
                config.d,
                total_bits,
                bits_per_value,
                wins,
                ties,
                losses,
                win_percent,
                tie_percent,
                loss_percent,
                format_ratio(summary.leb128_total_bits, summary.values, 6),
            ));
        }
    }
    out
}

fn render_markdown(summaries: &[SizeSummary]) -> String {
    let mut out = String::new();
    out.push_str("# Exact benchmark results\n\n");
    out.push_str(
        "Generated by `scripts/reproduce_paper.sh` from the canonical codec implementation. ",
    );
    out.push_str(
        "All rows are exact interval aggregates over the complete inclusive domain shown; ",
    );
    out.push_str("they are not Monte Carlo estimates or sparse samples.\n\n");

    out.push_str("## Average meaningful bits per value\n\n");
    out.push_str("| domain | values |");
    for profile in lotus::RECOMMENDED_PROFILES {
        out.push_str(&format!(" Lotus {} |", profile.label));
    }
    out.push_str(" LEB128 | VLQ | Elias γ | Elias δ |\n");
    out.push_str("|---|---:|");
    for _ in lotus::RECOMMENDED_PROFILES {
        out.push_str("---:|");
    }
    out.push_str("---:|---:|---:|---:|\n");

    for summary in summaries {
        out.push_str(&format!("| `{}` | {} |", summary.workload, summary.values));
        for config in &summary.lotus {
            let value = config
                .total_bits
                .map(|total| format_ratio(total, summary.values, 6))
                .unwrap_or_else(|| "NA".to_string());
            out.push_str(&format!(" {value} |"));
        }
        out.push_str(&format!(
            " {} | {} | {} | {} |\n",
            format_ratio(summary.leb128_total_bits, summary.values, 6),
            format_ratio(summary.vlq_total_bits, summary.values, 6),
            format_ratio(summary.elias_gamma_total_bits, summary.values, 6),
            format_ratio(summary.elias_delta_total_bits, summary.values, 6),
        ));
    }

    out.push_str("\n## Lotus versus LEB128\n\n");
    out.push_str("| domain | profile | wins | ties | losses | win % | tie % | loss % |\n");
    out.push_str("|---|---|---:|---:|---:|---:|---:|---:|\n");
    for summary in summaries {
        for config in &summary.lotus {
            if let Some(counts) = config.versus_leb128 {
                out.push_str(&format!(
                    "| `{}` | {} | {} | {} | {} | {} | {} | {} |\n",
                    summary.workload,
                    config.label,
                    counts.wins,
                    counts.ties,
                    counts.losses,
                    format_percent(counts.wins, summary.values),
                    format_percent(counts.ties, summary.values),
                    format_percent(counts.losses, summary.values),
                ));
            }
        }
    }

    out.push_str("\n## Framing caveat\n\n");
    out.push_str("Lotus figures are meaningful packed bits. Independently padding every codeword ");
    out.push_str("to a byte discards the byte-quantization advantage; use `EncodedLotus.bit_len` ");
    out.push_str("or the streaming `BitWriter`/`BitReader` APIs.\n");
    out
}

fn run_benchmark(format: OutputFormat, output: Option<String>) -> Result<(), LotusError> {
    let summaries = summarize_standard_domains();
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
