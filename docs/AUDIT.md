# Maintainer audit: canonical-format convergence

Date: 2026-08-18

## Resolved critical issues

1. **Multiple incompatible mapping semantics**
   - The payload and width descriptors now use separate canonical nonnegative and positive mappings.
   - Prototype shifted variants are removed rather than retained behind flags.

2. **Stale range recurrence**
   - Encoding validity is derived by constructing the actual width chain.
   - Profile range reporting uses the same positive descriptor recurrence.
   - Tests lock J1D1, J2D1, J1D2, and J3D1 boundaries.

3. **Misleading uniform-domain evidence**
   - Complete `u32` and `u64` claims now use exact interval aggregation.
   - The 94.485113% J1D2 win rate against LEB128 is an exact regression.

4. **Packed-bit versus padded-byte confusion**
   - Examples and Criterion benchmarks use the streaming bit writer/reader.
   - Documentation explicitly separates meaningful bits from backing bytes.

5. **Demo and documentation drift**
   - `docs/demo-fixture.js` is generated from Rust.
   - The HTML JavaScript port verifies every fixture case at page load.
   - Generated artifact freshness is enforced by CI.

## Remaining research work

- Add arbitrary-precision decoding to match the existing BigUint encoder.
- Add fuzz targets for malformed packed streams.
- Publish machine-specific Criterion baselines only with hardware/toolchain metadata.
