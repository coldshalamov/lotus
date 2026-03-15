#!/usr/bin/env bash
set -euo pipefail

cargo run --quiet --features cli --bin lotus -- benchmark --format markdown --output docs/RESULTS.md
cargo run --quiet --features cli --bin lotus -- benchmark --format json --output docs/results.json

echo "Regenerated docs/RESULTS.md and docs/results.json from source workloads."
