#!/usr/bin/env bash
set -euo pipefail

cargo bench --bench comparison -- --quiet || true

{
    echo "# Benchmark results snapshot"
    echo
    echo "Regenerated via \`scripts/reproduce_paper.sh\`."
    echo
    cargo run --example size_report
} > docs/RESULTS.md

echo "Updated docs/RESULTS.md with benchmark tables"
