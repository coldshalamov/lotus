# Canonical codec audit

Date: 2026-08-18

## Finding

The repository had accumulated three subtly different definitions of Lotus:

1. the original whitepaper's nonnegative payload plus positive width-descriptor mapping,
2. a shifted implementation that applied `value + 1` before the payload mapping,
3. a later implementation that reused the nonnegative mapping for positive descriptors while retaining a stale range recurrence.

Benchmarks, tests, examples, and the HTML demo then sampled or duplicated different pieces of those definitions. The approximately 95% uniform-u32 result was real under the original specification, but the repository no longer proved or implemented that exact codec consistently.

## Resolution

Version 0.2 establishes one wire format:

- nonnegative payload width: `floor(log2(n + 2))`,
- positive descriptor width: `floor(log2(v + 1))`,
- range derivation from the same positive descriptor mapping,
- no compatibility decoder for prototype byte layouts.

`src/lib.rs` is the sole implementation source. `src/metrics.rs`, the CLI, Criterion benchmarks, examples, generated results, and the HTML demo all consume the same named profile frontier.

## Evidence hardening

- Complete u32 and u64 claims use exact interval aggregation, not sparse samples.
- Fixed regression totals pin strict wins, ties, losses, and aggregate bits.
- Generated Markdown, JSON, and the demo fixture are regenerated together.
- The HTML demo verifies its JavaScript port against a Rust-generated boundary oracle.
- Streaming examples and benchmarks compose codewords with `BitWriter`/`BitReader`, preserving meaningful bits across byte boundaries.

## Remaining research, not correctness debt

- Profile selection for empirical non-uniform distributions can be built on the existing generic `(J,d)` API.
- A future escape mode could make the family formally unbounded beyond the configured envelope.
- BigUint decoding remains a possible extension; BigUint encoding already uses the canonical mapping.
