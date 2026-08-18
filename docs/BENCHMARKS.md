# Benchmark methodology and reproducibility

Lotus keeps deterministic size evidence separate from runtime throughput.

## Exact size evidence

Run:

```bash
scripts/reproduce_paper.sh
```

This regenerates:

- `docs/RESULTS.md`
- `docs/results.json`
- `docs/demo-fixture.js`

The size report is computed analytically by partitioning each inclusive integer domain at every point where Lotus, LEB128/VLQ, Elias gamma, or Elias delta can change length. Each interval is aggregated in constant time.

The complete `u32` and `u64` rows therefore cover every value exactly. They are not sparse grids and not Monte Carlo estimates.

Only the recommended profile frontier is reported:

- J1D1: tiny-range minimum overhead.
- J2D1: one-tier density through `2^31 - 3`.
- J1D2: minimum-bit full-`u64` profile.
- J3D1: one-tier full-`u64` profile.

## Meaningful bits versus backing bytes

Lotus is bit-oriented. Generated size tables count exact meaningful bits.

A standalone encode returns a byte vector whose final byte may contain zero padding. Comparing `bytes.len() * 8` per value measures a byte-framed protocol, not a packed Lotus stream. Use `EncodedLotus.bit_len`, `lotus_encoded_bit_len`, or the streaming writer.

## Runtime throughput

Run:

```bash
cargo bench --bench comparison
```

Criterion uses finite deterministic workloads and measures:

- packed Lotus stream encoding and decoding,
- LEB128 and VLQ,
- Elias gamma and delta.

Runtime numbers remain under `target/criterion/` and are not committed as portable facts.

## Drift prevention

`scripts/check_generated.sh` regenerates every artifact and fails if any committed output changes. The HTML demo also verifies its JavaScript math against the Rust-generated boundary fixture on page load.
