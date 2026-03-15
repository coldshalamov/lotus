# Maintainer Audit (trust and release readiness)

Date: 2026-03-15

This audit ranks issues by **severity** and **user trust impact**.

## Critical

1. **Benchmark evidence was hardcoded and non-reproducible**
   - `scripts/reproduce_paper.sh` previously wrote fixed markdown numbers without deriving them from benchmark code.
   - README and docs presented snapshot tables as measured evidence.
   - Impact: direct credibility loss.

2. **Repository metadata used placeholder URLs**
   - `Cargo.toml` contained `example/...` repository/homepage/docs links.
   - Impact: crate presentation looked templated/fabricated.

## High

3. **API ambiguity for bit-oriented framing**
   - Core encoder returned `Vec<u8>` with no explicit bit length in type; this risks framing confusion when composing streams.

4. **CI over-claimed guarantees**
   - Workflow posted benchmark comments and deployed pages directly from CI while benchmark artifacts were not generated truthfully.

5. **Docs drift and conflicting benchmark narratives**
   - README/BENCHMARKS/RESULTS/WHITEPAPER contained inconsistent and partially unverifiable figures.

## Medium

6. **Dependency hygiene**
   - CLI dependencies (`clap`, `hex`) were always included in main dependency graph.

7. **Small-int fast path surfaced as public API but asymmetrically documented**
   - Exposed optimization with no matching decode API created format/expectation ambiguity.

8. **Malformed-input and adversarial test coverage gaps**
   - Baseline tests existed, but lacked stronger truncation/trailing-bit/framing adversarial checks.

## Implemented priorities in this pass

1. Restore benchmark trust via generated artifacts + drift checks.
2. Fix metadata professionalism and docs consistency.
3. Clarify API framing with explicit encoded representation.
4. Tighten CI to reflect verifiable guarantees.
5. Add additional correctness/adversarial tests.

## Remaining follow-up candidates

- Add decode support for `BigUint` (currently encode-only API).
- Add cargo-fuzz targets and seed corpus committed under `fuzz/`.
- Add scheduled benchmark workflow for long-running throughput publication.
