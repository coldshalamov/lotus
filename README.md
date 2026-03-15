# Lotus 🪷

[![CI](https://github.com/lotus-codec/lotus/actions/workflows/ci.yml/badge.svg)](https://github.com/lotus-codec/lotus/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/lotus.svg)](https://crates.io/crates/lotus)
[![docs.rs](https://docs.rs/lotus/badge.svg)](https://docs.rs/lotus)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Lotus is a parametric bit-level integer codec for Rust. It reclaims representational density by mapping every fixed-width bitstring into contiguous integer ranges, then restores self-delimiting behavior using a bounded tier chain and jumpstarter.

## Project status

This crate is **experimental** and actively hardened for reproducibility and correctness. Public APIs are small and documented, and benchmark claims are generated from code in this repository.

## Install

```bash
cargo add lotus
```

CLI:

```bash
cargo install --path . --features cli
```

## Library quick start

```rust
use lotus::{lotus_decode_u64, lotus_encode_u64_framed, LOTUS_J2D1};

let encoded = lotus_encode_u64_framed(42, LOTUS_J2D1.0, LOTUS_J2D1.1)?;
assert_eq!(encoded.bit_len, 9);

let (value, consumed_bits) = lotus_decode_u64(&encoded.bytes, LOTUS_J2D1.0, LOTUS_J2D1.1)?;
assert_eq!(value, 42);
assert_eq!(consumed_bits, encoded.bit_len);
# Ok::<(), lotus::LotusError>(())
```

## CLI examples

```bash
printf '42\n' | lotus encode --jumpstarter 2 --tiers 1 --with-bits
printf '2a\n' | lotus decode --jumpstarter 2 --tiers 1 --with-bits
```

Generate deterministic benchmark-size artifacts used in docs:

```bash
scripts/reproduce_paper.sh
```

## Benchmark evidence policy

- Runtime throughput is measured by Criterion (`cargo bench --bench comparison`).
- Size tables in docs are generated from deterministic workload code (`src/metrics.rs`) and committed artifacts (`docs/RESULTS.md`, `docs/results.json`).
- No hand-written benchmark snapshot tables are accepted.

See [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md) and [`docs/RESULTS.md`](docs/RESULTS.md).

## Documentation

- [API guide](docs/API.md)
- [Theory](docs/THEORY.md)
- [Benchmarks](docs/BENCHMARKS.md)
- [Results (generated)](docs/RESULTS.md)
- [Maintainer audit](docs/AUDIT.md)
- [Whitepaper notes](docs/WHITEPAPER.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Run `cargo fmt`, `cargo clippy`, `cargo test`, and `scripts/check_generated.sh` before opening a PR.
