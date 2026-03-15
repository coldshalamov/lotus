#!/usr/bin/env bash
set -euo pipefail

before_results="$(sha256sum docs/RESULTS.md 2>/dev/null | awk '{print $1}')"
before_json="$(sha256sum docs/results.json 2>/dev/null | awk '{print $1}')"

scripts/reproduce_paper.sh

after_results="$(sha256sum docs/RESULTS.md | awk '{print $1}')"
after_json="$(sha256sum docs/results.json | awk '{print $1}')"

if [[ "$before_results" != "$after_results" || "$before_json" != "$after_json" ]]; then
  echo "Generated benchmark artifacts changed after regeneration. Commit regenerated files." >&2
  exit 1
fi
