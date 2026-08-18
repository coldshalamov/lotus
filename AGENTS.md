# Maintainer instructions for automated contributors

## Canonical codec invariant

There is one Lotus format in this repository.

- Nonnegative payload width: `floor(log2(n + 2))`.
- Positive descriptor width: `floor(log2(v + 1))`.
- Positive descriptors map width `L` onto values `2^L - 1 ..= 2^(L+1) - 2`.
- `src/lib.rs` is the implementation source of truth.
- Do not introduce compatibility branches, alternate mapping helpers, or duplicated range recurrences.

Any mapping change is a wire-format change and must update `docs/FORMAT.md`, exact-domain regression tests, generated benchmark artifacts, and the demo fixture together.

## Trust and reproducibility

- Never commit hand-written benchmark numbers as measured evidence.
- Regenerate artifacts with `scripts/reproduce_paper.sh`.
- Run `scripts/check_generated.sh` before finalizing.
- Complete-domain claims must use exact interval aggregation in `src/metrics.rs`, not sparse samples.

## Framing

- Lotus evidence counts meaningful packed bits.
- Prefer `EncodedLotus` or streaming `BitWriter`/`BitReader` APIs.
- Do not concatenate independently padded `Vec<u8>` codewords and call the result a packed stream.

## Profile policy

Promote only the profiles in `RECOMMENDED_PROFILES`. Generic `(J,d)` support may remain, but dominated configurations must not reappear in benchmark tables or the HTML demo without evidence that they extend the Pareto frontier.

## CI

CI must validate formatting, linting, default/all-feature tests, generated artifact freshness, and a Criterion smoke test.
