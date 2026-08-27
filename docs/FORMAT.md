# Canonical Lotus format

This document is normative for the Lotus wire mapping implemented by this repository.

## 1. Two mappings, one codec

Lotus distinguishes the nonnegative integer payload from the positive integers used to describe widths.

For a nonnegative integer `n`:

\[
L_0(n) = \lfloor \log_2(n + 2) \rfloor
\]

An `L`-bit nonnegative payload represents the consecutive interval:

\[
2^L - 2 \;\ldots\; 2^{L+1} - 3
\]

and stores:

\[
p = n - (2^L - 2)
\]

For a positive width value `v >= 1`:

\[
L_+(v) = \lfloor \log_2(v + 1) \rfloor
\]

An `L`-bit positive descriptor represents:

\[
2^L - 1 \;\ldots\; 2^{L+1} - 2
\]

and stores:

\[
p = v - (2^L - 1)
\]

These formulas are not interchangeable. The payload is nonnegative; every width descriptor is positive.

## 2. Codeword layout

For configuration `(J, d)`:

1. Compute the payload width `L0`.
2. Compute `Li+1 = L+(Li)` for `i = 0 .. d-1`.
3. Store `Ld - 1` in the fixed `J`-bit jumpstarter.
4. For `i = d .. 1`, store the positive value `Li-1` in an `Li`-bit descriptor.
5. Store `n` in the final `L0`-bit nonnegative payload.

The meaningful bit length is:

\[
B_{J,d}(n) = J + \sum_{i=0}^{d} L_i
\]

Fields are written MSB-first. A standalone byte buffer pads the final byte with zeroes; those padding bits are not part of the codeword.

## 3. Range derivation

The jumpstarter can describe an outer width through `2^J`. A positive descriptor of width `w` can describe a next width through:

\[
2^{w+1} - 2
\]

Applying that recurrence `d` times gives the maximum payload width.

The recommended frontier is:

| profile | `(J,d)` | maximum payload width | maximum `u64` value | purpose |
|---|---:|---:|---:|---|
| J1D1 | `(1,1)` | 6 | 125 | minimum tiny-range overhead |
| J2D1 | `(2,1)` | 30 | `2^31 - 3` | best one-tier density within that range |
| J1D2 | `(1,2)` | 126 | `u64::MAX` | minimum meaningful bits over full `u64` |
| J3D1 | `(3,1)` | 510 | `u64::MAX` | one-tier full-`u64` traversal |

Other `(J,d)` values remain accepted by the generic API, but dominated profiles are intentionally absent from benchmark tables and the demo.

## 4. Canonicality and compatibility

Each fixed-width field maps exactly all bitstrings of that width onto one consecutive interval. Therefore every accepted field is already minimally and canonically encoded.

This corrected mapping is wire-incompatible with prototype revisions that:

- applied `value + 1` before the nonnegative payload mapping,
- used the nonnegative mapping for positive width descriptors, or
- propagated stale range recurrences ending in `-3` or `-4`.

No compatibility decoder for those experimental variants is retained. Lotus `0.2` contains one format only.

## 5. Source-of-truth rule

`src/lib.rs` owns the codec math. Everything else must derive from it:

- exact size evidence: `src/metrics.rs`,
- generated tables: `docs/RESULTS.md` and `docs/results.json`,
- demo fixture: `docs/demo-fixture.js`,
- HTML self-check: `docs/index.html`,
- regression constants: `tests/lotus_tests.rs`.

Changes to the mapping require all generated artifacts and exact-domain tests to change together.
