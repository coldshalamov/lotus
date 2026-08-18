# Contributing to Lotus

Lotus has one canonical wire mapping. Read `docs/FORMAT.md` and the repository `AGENTS.md` before changing codec logic.

Required checks:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --all-features
scripts/check_generated.sh
cargo bench --bench comparison -- --sample-size 10
```

When codec math, profiles, metrics, or demo behavior changes, regenerate and commit:

- `docs/RESULTS.md`
- `docs/results.json`
- `docs/demo-fixture.js`

Complete-domain claims must use exact interval aggregation. Do not replace exact `u32`/`u64` evidence with random samples or sparse grids.

Keep bit framing explicit. A standalone byte vector may contain final-byte padding; packed-stream examples must use `BitWriter` and `BitReader`.
