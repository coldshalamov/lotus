#!/usr/bin/env bash
set -euo pipefail

files=(
  docs/RESULTS.md
  docs/results.json
  docs/demo-fixture.js
)

declare -A before
for file in "${files[@]}"; do
  before["$file"]="$(sha256sum "$file" 2>/dev/null | awk '{print $1}')"
done

scripts/reproduce_paper.sh

changed=0
for file in "${files[@]}"; do
  after="$(sha256sum "$file" | awk '{print $1}')"
  if [[ "${before[$file]}" != "$after" ]]; then
    echo "Generated artifact changed after regeneration: $file" >&2
    changed=1
  fi
done

if [[ "$changed" -ne 0 ]]; then
  echo "Commit the regenerated artifacts." >&2
  exit 1
fi
