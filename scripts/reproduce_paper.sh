#!/usr/bin/env bash
set -euo pipefail

cargo run --quiet --features cli --bin lotus -- benchmark --format markdown --output docs/RESULTS.md
cargo run --quiet --features cli --bin lotus -- benchmark --format json --output docs/results.json
cargo run --quiet --example generate_demo_fixture > docs/demo-fixture.js

echo "Regenerated exact results and the Rust-backed demo fixture."
