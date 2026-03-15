# Theory: where Lotus fits

Lotus sits between dense fixed-width encodings and self-delimiting universal codes.
By unfolding fixed-width bitstrings into consecutive integer ranges, it reclaims density while maintaining prefix-decodability through a bounded tier chain.

## Core properties

- **Density reclaiming:** every distinct bitstring is assigned to a unique integer in contiguous ranges.
- **Configurable envelope:** `(J, d)` determines representable range and overhead profile.
- **Bit-level framing:** Lotus is naturally bit-oriented; exact bit length matters for stream composition.

## Trade-offs

- Larger `J` increases fixed per-value header cost but expands immediate width state.
- Larger `d` expands maximum range through recursive width descriptions, at added header complexity.

## Evidence and claims

For empirical size/speed claims, use reproducible artifacts and benchmark outputs documented in:

- `docs/BENCHMARKS.md`
- `docs/RESULTS.md`
