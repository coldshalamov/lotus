# Changelog

## Unreleased

### Added
- Added real round-trip comparator codecs in `src/metrics.rs`: VLQ (big-endian base-128), Elias gamma (encode/decode), and a full Elias delta (encode/decode, upgrading the prior length-only helper). Each ships with a no-alloc bit-length helper.
- Added decode-throughput benchmarks. `benches/comparison.rs` now measures both `decode_*` and `encode_*` across four Lotus `(J, d)` configs and all four comparators (LEB128, VLQ, Elias γ, Elias δ).
- Added `examples/generate_demo_fixture.rs`, which emits a Rust-generated reference fixture consumed by the interactive demo's self-verification.
- Added a redesigned, self-contained interactive demo at `docs/index.html`: 20 Hz value counter, live SVG size-comparison bar chart, multi-config `(J, d)` support, live encoding-layout breakdown, and on-load self-verification against the Rust reference fixture.
- Expanded `SizeSummary` and the CLI `benchmark` output to cover four Lotus configs and four comparators.

### Changed
- Replaced the linear `lotus_width_for_value` search with an O(1) leading-zeros closed form, proven equivalent by an exhaustive test over `[0, 2^24)` plus random u128 samples.
- Added a u64 decode fast path (`lotus_decode_fixed_u64`) that keeps tier-chain and sub-64-bit payload decodes in u64, avoiding u128 widening. Measured 36–45% decode throughput improvement (local Criterion).
- Lotus decode now compares against LEB128, VLQ, Elias γ, and Elias δ rather than LEB128 + Elias-delta-length only.

### Changed (prior)
- Added reproducible benchmark artifact pipeline (`scripts/reproduce_paper.sh`) and drift check (`scripts/check_generated.sh`).
- Added `EncodedLotus` and `lotus_encode_u64_framed` to expose exact encoded bit length.
- Moved CLI dependencies behind the `cli` feature and made binary require that feature.
- Reworked CLI benchmark command to emit markdown/csv/json from repository workloads.
- Removed non-reproducible hardcoded benchmark tables from maintained docs.
- Tightened tests with framing and truncation adversarial checks.
