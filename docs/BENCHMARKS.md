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

### Workloads

- `small`: `0..=255`
- `medium`: `0..1_000_000` sampled every `10_000`
- `large32`: `0..4_000_000_000` sampled every `25_000_000`

### Codecs compared

- **Lotus** in four `(J, d)` configurations: J1D2, J2D1, J3D1, J3D2.
- **LEB128** — little-endian base-128 varint (protobuf / DWARF / WebAssembly).
- **VLQ** — big-endian base-128 varint (MIDI / Bitcoin).
- **Elias γ** — bit-oriented universal code.
- **Elias δ** — bit-oriented universal code.

All comparators are real round-trip codecs implemented in `src/metrics.rs` (encode + decode + exact bit-length helpers), covered by round-trip and exact-bit-length tests in `tests/lotus_tests.rs`.

### Reading the size table

Lotus and the Elias codes are **bit-oriented**: their length grows smoothly with the
magnitude of the value. LEB128 and VLQ are **byte-oriented**: their length jumps in
8-bit steps at every `×128` boundary. So:

- For small values, Lotus's per-value header overhead is amortized and it wins on density.
- At medium magnitudes, Lotus and the byte varints are roughly even.
- For large values, Lotus (with enough tiers) pulls ahead again because it never rounds
  up to a whole byte.

## 2) Runtime throughput measurements

Throughput is measured via Criterion:

```bash
cargo bench --bench comparison
```

Criterion output is written under `target/criterion/` and is intentionally not committed.

### Decode and encode benchmarks

`benches/comparison.rs` measures both directions:

- `decode_{workload}` — pre-encode a batch, then bench the decode loop.
- `encode_{workload}` — bench the encode loop.

Each group covers all four Lotus configs plus LEB128, VLQ, Elias γ, and Elias δ.

### Honest comparison caveats

- **Lotus is bit-oriented; LEB128/VLQ are byte-oriented.** Byte varints read whole bytes
  with shift/mask and no cross-byte bit packing, so they are inherently cheaper per value
  on raw `ns/value`. Lotus and the Elias codes share the bit-packing cost.
- Throughput depends on CPU/toolchain/runtime settings and must be reproduced locally.
- Claims should cite generated artifacts or Criterion outputs, not hand-edited numbers.

## Interactive demo

`docs/index.html` is a self-contained, offline interactive page that recomputes Lotus
bit lengths in JavaScript (a faithful port of `src/lib.rs`) and self-verifies against a
Rust-generated reference fixture on every page load. The fixture is produced by:

```bash
cargo run --example generate_demo_fixture
```

The demo shows **size** only (deterministic). It deliberately does **not** display runtime
throughput, which is not reproducible from a static page.

## Important caveats

- Deterministic docs tables represent **size** only, not speed.
- Throughput depends on CPU/toolchain/runtime settings and must be reproduced locally.
- Claims should cite generated artifacts or Criterion outputs, not hand-edited markdown numbers.
