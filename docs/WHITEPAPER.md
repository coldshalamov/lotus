# Lotus whitepaper notes

This document summarizes the codec design rationale and mathematical intuition.

For reproducible empirical claims, use generated artifacts instead of fixed tables in prose:

- `docs/RESULTS.md`
- `docs/results.json`
- `target/criterion/` (local throughput runs)

## Scope

- Canonical mapping from fixed-width bitstrings to consecutive integer ranges.
- Tiered length-chain encoding with jumpstarter anchor.
- Configuration trade-offs for `(J, d)`.

## Reproducibility policy

Any numeric claim in this repository should be traceable to:

1. source workloads in `src/metrics.rs`,
2. generated artifacts in `docs/RESULTS.md` / `docs/results.json`, or
3. locally produced Criterion reports.

Historical narrative text from earlier prototype phases has been intentionally removed to avoid stale or non-reproducible benchmark claims.
