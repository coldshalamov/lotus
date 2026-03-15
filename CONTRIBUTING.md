# Contributing to Lotus

Thanks for contributing.

## Required checks before PR

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --all-features
scripts/check_generated.sh
```

If you changed benchmark workloads or benchmark-report formatting, include regenerated:

- `docs/RESULTS.md`
- `docs/results.json`

## Benchmark claim policy

Do not hand-edit benchmark result tables presented as measured output.
All benchmark-size claims must come from generated artifacts.

## Design policy

- Preserve Lotus mapping/invariants unless correctness evidence requires changes.
- Prefer explicit framing (`bit_len`) in external protocol examples.
- Keep the core library free of unnecessary CLI dependencies.
