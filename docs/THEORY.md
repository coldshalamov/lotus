# Theory: where Lotus fits

Lotus is a bounded, self-describing integer code built from a dense fixed-width primitive.

The primitive reclaims leading-zero aliases by assigning every bitstring of width `L` to a distinct integer in one consecutive interval. A recursive chain of positive width descriptors restores prefix decoding. A fixed `J`-bit jumpstarter anchors that chain.

The key distinction is structural:

- payload integers are nonnegative and use `floor(log2(n + 2))`;
- width descriptors are positive and use `floor(log2(v + 1))`.

That one-bit shift is the hinge of the format. Conflating the two mappings changes code lengths, range limits, and encoded bytes.

Increasing `J` buys a wider outer state at fixed per-value cost. Increasing `d` buys range recursively at the cost of another descriptor. The useful `u64` profiles form a small Pareto frontier rather than an indiscriminate grid; see `docs/FORMAT.md`.

Lotus's density advantage over LEB128 is a packed-bitstream advantage. LEB128 rounds each value to whole bytes. Lotus does not. If every Lotus value is independently padded to a byte, much of that advantage is intentionally discarded.

Exact empirical evidence is generated from interval aggregation in `src/metrics.rs`, not random samples or hand-written tables.
