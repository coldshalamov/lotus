# Changelog

## Unreleased

### Breaking
- Replaced the prototype shifted mapping with the canonical Lotus format: nonnegative payloads use `floor(log2(n + 2))`; positive width descriptors use `floor(log2(v + 1))`.
- Removed compatibility with the old experimental wire variants. Existing prototype bytes must be re-encoded.
- Changed the CLI default from J2D1 to the full-domain minimum-bit J1D2 profile.
- Removed the unused `small-int-fastpath` feature.

### Added
- Added `LotusConfig`, named recommended profiles, and a single shared profile frontier.
- Added exact interval aggregation over complete `u32` and `u64` domains.
- Added exact strict-win/tie/loss regressions against LEB128.
- Added a generated JavaScript demo fixture checked against the Rust implementation.
- Added an exact shared-budget browser race, interactive canonical codeword inspector, and complete-`u32` evidence presentation.
- Added a dependency-free Node contract check that rejects browser/Rust fixture drift and hand-authored benchmark constants.
- Added `docs/FORMAT.md` as the normative wire specification.

### Fixed
- Corrected positive descriptor mapping and configuration range derivation.
- Removed the stale duplicated maximum-width recurrence.
- Fixed the streaming example and Criterion benchmarks to use packed bitstreams rather than concatenated padded buffers.
- Made Elias gamma and delta comparator codecs round-trip `u64::MAX`.
- Removed dominated J3D2 from promoted profiles and generated evidence.
- Restored the intended counter-and-chart demo without restoring the prototype `value + 1` payload shift or stale `-4` range recurrence.
