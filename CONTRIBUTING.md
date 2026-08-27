# Contributing to Lotus

## Required checks

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --all-features
scripts/check_generated.sh
```

If codec math, profile metadata, benchmark rendering, or demo behavior changes, run:

```bash
scripts/reproduce_paper.sh
```

and commit all three generated artifacts:

- `docs/RESULTS.md`
- `docs/results.json`
- `docs/demo-fixture.js`

## Format discipline

`docs/FORMAT.md` is normative and `src/lib.rs` is the implementation source of truth.

Do not add alternate payload/descriptor mappings, compatibility branches for prototype bytes, or a second range recurrence. A wire-format change must update the specification, exact-domain regressions, generated evidence, and demo fixture in the same pull request.

## Framing discipline

Lotus is bit-oriented. Use `EncodedLotus.bit_len` or the streaming `BitWriter`/`BitReader` APIs when measuring or composing codewords. Per-value padded byte lengths are a different protocol measurement and must be labeled as such.
