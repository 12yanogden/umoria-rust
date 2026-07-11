#!/usr/bin/env bash
# RNG invariant golden equals exactly 1043618065.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
F="$ROOT/tests/golden/rng/z10001.txt"

[[ -f "$F" ]] || { echo "FAIL: $F missing" >&2; exit 1; }
val="$(tr -d '[:space:]' < "$F")"
if [[ "$val" != "1043618065" ]]; then
 echo "FAIL: z10001 = '$val', expected 1043618065" >&2
 exit 1
fi
echo "PASS: z10001 == 1043618065"
