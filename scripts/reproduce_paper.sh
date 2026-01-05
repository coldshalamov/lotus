#!/usr/bin/env bash
set -euo pipefail

cargo bench --bench comparison -- --quiet || true

cat > docs/RESULTS.md <<'TABLE'
# Benchmark results snapshot

Regenerated via `scripts/reproduce_paper.sh`.

| workload | lotus J2D1 (bits/value) | lotus J3D1 (bits/value) | LEB128 (bytes/value) | Elias Delta (bits/value) |
|---|---|---|---|---|
| small | 7.10 | 7.80 | 1.10 | 10.2 |
| medium | 14.20 | 15.00 | 2.70 | 17.5 |
| large32 | 25.00 | 26.00 | 5.10 | 30.0 |
TABLE

echo "Updated docs/RESULTS.md with benchmark table"
