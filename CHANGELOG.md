# Changelog

## Unreleased

### Changed
- Added reproducible benchmark artifact pipeline (`scripts/reproduce_paper.sh`) and drift check (`scripts/check_generated.sh`).
- Added `EncodedLotus` and `lotus_encode_u64_framed` to expose exact encoded bit length.
- Moved CLI dependencies behind the `cli` feature and made binary require that feature.
- Reworked CLI benchmark command to emit markdown/csv/json from repository workloads.
- Removed non-reproducible hardcoded benchmark tables from maintained docs.
- Tightened tests with framing and truncation adversarial checks.
