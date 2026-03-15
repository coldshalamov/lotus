# Maintainer instructions for automated contributors

## Trust and reproducibility

- Never commit hand-written benchmark numbers as measured evidence.
- Regenerate benchmark artifacts with `scripts/reproduce_paper.sh`.
- Run `scripts/check_generated.sh` before finalizing.

## Claims policy

- Any numeric benchmark claim in docs must be traceable to:
  - `src/metrics.rs` workloads + generated `docs/RESULTS.md` / `docs/results.json`, or
  - local Criterion outputs under `target/criterion/`.

## API policy

- Prefer framed APIs (`EncodedLotus` / bit length aware usage) in docs/examples.
- Preserve Lotus mapping and decoding invariants unless tests prove correctness issues.

## CI policy

- CI should validate formatting, linting, tests, and generated artifact freshness.
- CI must not imply guarantees not reproducible from repository code.
