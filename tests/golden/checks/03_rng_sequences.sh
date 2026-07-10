#!/usr/bin/env bash
# Phase 1.4.2 test 3: per-seed rnd() sequence goldens exist, are non-empty,
# seed-1 has >= 10001 lines, and capture is bit-stable across two runs.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
RNG="$ROOT/tests/golden/rng"

for seed in 1 42 12345 2147483647; do
    f="$RNG/rnd_seed${seed}.txt"
    [[ -s "$f" ]] || { echo "FAIL: $f missing or empty" >&2; exit 1; }
done

lines=$(wc -l < "$RNG/rnd_seed1.txt")
if (( lines < 10001 )); then
    echo "FAIL: rnd_seed1.txt has $lines lines, expected >= 10001" >&2
    exit 1
fi

# Determinism: re-capture and compare hashes of every rng golden.
before=$(cat "$RNG"/*.txt | shasum -a 256 | awk '{print $1}')
"$ROOT/tools/capture/capture_rng.sh" >/dev/null
after=$(cat "$RNG"/*.txt | shasum -a 256 | awk '{print $1}')
if [[ "$before" != "$after" ]]; then
    echo "FAIL: RNG capture not bit-stable across runs ($before != $after)" >&2
    exit 1
fi

echo "PASS: rnd() sequences present, seed-1 has $lines lines, capture is stable"
