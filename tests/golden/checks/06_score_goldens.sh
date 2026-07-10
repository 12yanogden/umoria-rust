#!/usr/bin/env bash
# Phase 1.4.4 test 6: score goldens non-empty & stable (masked birth_date).
#
# Deterministic score goldens:
#   scores/scores_initial.dat  - pristine committed high-score file (a real
#                                HighScore_t record: "test", Human Mage, 47 pts).
#                                Begins with valid game version bytes; the record
#                                birth_date (a clock-derived field) lives at
#                                decoded offset 8:4 (record0 = 3 + 1 xor-seed + 4
#                                points => birth_date at 8, len 4; see
#                                game_save.cpp saveHighScore / scores.cpp).
#   scores/scores_screen.txt   - `umoria -s 42 -d` (showScoresScreen) output.
#                                Fully deterministic (the display shows no
#                                timestamps), so it is byte-stable across re-runs.
#
# This check validates: (a) the goldens exist / are well-formed, (b) the
# `-d` screen is stable across re-capture, and (c) compare_masked.py correctly
# masks the score birth_date (positive + negative control) so a freshly written
# score file could be compared modulo birth_date.
#
# NOTE (documented limitation): a live scripted-death score-write golden
# (scores_<scenario>.dat produced by recordNewHighScore) is intentionally
# deferred. Recording a new score requires the character to *die* in-game, which
# means searching for a long, brittle deterministic keystroke death sequence --
# an open-ended task out of proportion to this capture leaf. The score WRITE byte
# format is already pinned by scores_initial.dat (a genuine record) + the
# masked-comparison mechanism proven below, and the identical clock field
# (date_of_birth) is exercised byte-for-byte by the save golden (check 05). The
# exact birth_date offset formula (8 + 64*N, len 4) is recorded for the manifest.
# Runs the reference binary under a pty; likely needs required_permissions:["all"].
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SCDIR="$ROOT/tests/golden/scores"
INIT="$SCDIR/scores_initial.dat"
SCREEN="$SCDIR/scores_screen.txt"
CMP="$ROOT/tools/capture/compare_masked.py"

# (a) initial score file well-formed
[[ -s "$INIT" ]] || { echo "FAIL: $INIT missing or empty" >&2; exit 1; }
read -r maj min pat < <(python3 - "$INIT" <<'PY'
import sys
c=open(sys.argv[1],'rb').read()
print(c[0], c[1], c[2])
PY
)
if [[ "$maj" != "5" || "$min" -lt 2 || "$min" -gt 7 ]]; then
    echo "FAIL: scores_initial.dat version bytes invalid ($maj.$min.$pat)" >&2
    exit 1
fi

# (b) -d screen exists, non-empty, stable across re-capture (hash compare)
[[ -s "$SCREEN" ]] || { echo "FAIL: $SCREEN missing or empty" >&2; exit 1; }
h1=$(shasum -a 256 < "$SCREEN" | awk '{print $1}')

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
python3 "$ROOT/tools/capture/pty_driver.py" \
    --binary "$ROOT/umoria/umoria" \
    --seed 42 --extra-arg=-d \
    --keys "$ROOT/tests/golden/transcripts/scores_screen.keys" \
    --raw "$TMP/s.raw" \
    --screen "$TMP/s.screen" \
    --cwd "$ROOT/umoria" \
    --char-delay 0.2 \
    --timeout 15
[[ -s "$TMP/s.screen" ]] || { echo "FAIL: -d re-capture produced empty screen" >&2; exit 1; }
h2=$(shasum -a 256 < "$TMP/s.screen" | awk '{print $1}')
if [[ "$h1" != "$h2" ]]; then
    echo "FAIL: -d score screen not stable across re-run ($h1 != $h2)" >&2
    exit 1
fi

# (c) masked comparator: mask the birth_date (offset 8:4) of the first record.
#     Positive control: file vs itself, masked -> EQUAL.
python3 "$CMP" --scheme score --mask 8:4 "$INIT" "$INIT" >/dev/null || {
    echo "FAIL: masked self-compare of scores_initial.dat did not report EQUAL" >&2; exit 1; }

#     Negative control: flip a birth_date byte -> DIFFER without mask, EQUAL with mask.
python3 - "$INIT" "$TMP/mutated.dat" <<'PY'
import sys
c=bytearray(open(sys.argv[1],'rb').read())
# ciphertext byte 8 XORed into decoded birth_date; flipping it changes only the
# decoded birth_date byte and cascades in ciphertext -- exactly the volatile case.
c[8]^=0xFF
open(sys.argv[2],'wb').write(c)
PY
if python3 "$CMP" --scheme score "$INIT" "$TMP/mutated.dat" >/dev/null 2>&1; then
    echo "FAIL: negative control -- mutated birth_date compared EQUAL without mask" >&2; exit 1
fi
python3 "$CMP" --scheme score --mask 8:4 "$INIT" "$TMP/mutated.dat" >/dev/null || {
    echo "FAIL: masking birth_date did not neutralize the mutation" >&2; exit 1; }

echo "PASS: score goldens well-formed, -d screen stable, birth_date masking verified"
