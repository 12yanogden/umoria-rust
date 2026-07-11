#!/usr/bin/env bash
# normal_table golden exists with exactly 256 entries.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
F="$ROOT/tests/golden/rng/normal_table.txt"

[[ -s "$F" ]] || { echo "FAIL: $F missing or empty" >&2; exit 1; }
n=$(grep -c . "$F")
if (( n != 256 )); then
 echo "FAIL: normal_table has $n entries, expected 256" >&2
 exit 1
fi
echo "PASS: normal_table has 256 entries"
