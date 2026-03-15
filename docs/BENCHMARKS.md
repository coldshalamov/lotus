# Benchmark methodology and reproducibility

Lotus separates benchmark evidence into two categories:

1. **Deterministic size statistics** (used for docs claims)
2. **Runtime throughput measurements** (Criterion reports)

## 1) Deterministic size statistics

Deterministic benchmark-size outputs are generated from `src/metrics.rs` workloads and codecs.

Generate artifacts:

```bash
scripts/reproduce_paper.sh
```

This regenerates:

- `docs/RESULTS.md` (human-readable table)
- `docs/results.json` (machine-readable data)

CI checks these files for drift with `scripts/check_generated.sh`.

## 2) Runtime throughput measurements

Throughput is measured via Criterion:

```bash
cargo bench --bench comparison
```

Criterion output is written under `target/criterion/` and is intentionally not committed.

## Workloads (current)

- `small`: `0..=255`
- `medium`: `0..=1_000_000` sampled every `10_000`
- `large32`: `0..=4_000_000_000` sampled every `25_000_000`

## Comparators

- Lotus J2D1
- Lotus J3D1
- LEB128
- Elias Delta bit length

## Important caveats

- Deterministic docs tables represent **size** only, not speed.
- Throughput depends on CPU/toolchain/runtime settings and must be reproduced locally.
- Claims should cite generated artifacts or Criterion outputs, not hand-edited markdown numbers.
