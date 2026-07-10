#!/usr/bin/env bash
# Phase 1.4.4 test 5: save golden non-empty, well-formed & stable.
#
# The committed golden save/newchar_seed42.sav must be non-empty and begin with
# the 5.7.15 version bytes (05 07 0F). A fresh re-capture with the same seed +
# env must be identical to the golden OUTSIDE the documented clock-volatile
# plaintext ranges (save timestamp l and player date_of_birth), compared on the
# XOR-decoded plaintext via compare_masked.py.
#
# Volatile plaintext ranges for newchar_seed42 (living character):
#   3894:4  save timestamp l   (game_save.cpp:299, wrLong(getCurrentUnixTime()))
#   3910:4  date_of_birth      (game_save.cpp:309, wrLong(py.misc.date_of_birth))
# Runs the reference binary under a pty; likely needs required_permissions:["all"].
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SDIR="$ROOT/tests/golden/save"
NAME="newchar_seed42"
SEED=42
GOLD="$SDIR/$NAME.sav"
MASK=(--mask 3894:4 --mask 3910:4)

[[ -s "$GOLD" ]] || { echo "FAIL: golden $GOLD missing or empty" >&2; exit 1; }

# Version bytes 05 07 0F (5.7.15), written with xor_byte reset so they are plain.
ver=$(head -c 3 "$GOLD" | xxd -p)
if [[ "$ver" != "05070f" ]]; then
    echo "FAIL: golden does not begin with version bytes 05 07 0F (got $ver)" >&2
    exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

python3 "$ROOT/tools/capture/pty_driver.py" \
    --binary "$ROOT/umoria/umoria" \
    --seed "$SEED" \
    --save "$TMP/fresh.sav" \
    --keys "$ROOT/tests/golden/transcripts/$NAME.keys" \
    --raw "$TMP/fresh.raw" \
    --screen "$TMP/fresh.screen" \
    --cwd "$ROOT/umoria" \
    --char-delay 0.15 \
    --timeout 30

[[ -s "$TMP/fresh.sav" ]] || { echo "FAIL: fresh capture produced no save file" >&2; exit 1; }

if ! python3 "$ROOT/tools/capture/compare_masked.py" --scheme save "${MASK[@]}" "$GOLD" "$TMP/fresh.sav"; then
    echo "FAIL: save golden not stable outside volatile ranges" >&2
    exit 1
fi

echo "PASS: save golden well-formed (05 07 0F) and stable modulo timestamp/date_of_birth"
