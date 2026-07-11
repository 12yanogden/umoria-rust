#!/usr/bin/env bash
# PTY transcript capture.
#
# play.sh runs the Rust umoria binary under a pty with a fixed keystroke script
# (ending in an explicit ^X save+quit) and renders the raw pty bytes through a
# real terminal emulator (pyte) into a 24x80 final-screen dump.
#
# The screen must be:
# 1. non-empty,
# 2. stable ACROSS SEPARATE capture bursts (capture, sleep, capture again) —
# not merely across two back-to-back runs. The old escape-stripping
# concatenator was stable within a burst but drifted across bursts because
# it concatenated the (timing-dependent) raw byte stream instead of
# modeling cursor positioning; the pyte 24x80 render is chunk-independent.
# 3. bounded to <= 80 columns per line. The old concatenator jammed columns
# together into multi-hundred-character lines, so this guards against a
# regression back to it.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
TDIR="$ROOT/tests/golden/transcripts"
NAME="newchar_seed42"
SEED=42

[[ -f "$TDIR/$NAME.keys" ]] || { echo "FAIL: $TDIR/$NAME.keys missing" >&2; exit 1; }

"$ROOT/tools/capture/play.sh" "$NAME" "$SEED" >/dev/null
[[ -s "$TDIR/$NAME.screen" ]] || { echo "FAIL: $TDIR/$NAME.screen missing or empty" >&2; exit 1; }
h1=$(shasum -a 256 < "$TDIR/$NAME.screen" | awk '{print $1}')

# Bounded line width (<= 80 cols): a regression to the concatenator would
# produce lines hundreds of chars wide.
maxlen=$(awk '{ if (length($0) > m) m = length($0) } END { print m+0 }' "$TDIR/$NAME.screen")
if [[ "$maxlen" -gt 80 ]]; then
    echo "FAIL: screen has a $maxlen-column line (> 80); screen is not a 24x80 render" >&2
    exit 1
fi

# Separate capture burst: sleep so the wall clock and any timing-sensitive
# chunking differ from the first burst, then re-capture and require an identical
# render.
sleep 2
"$ROOT/tools/capture/play.sh" "$NAME" "$SEED" >/dev/null
h2=$(shasum -a 256 < "$TDIR/$NAME.screen" | awk '{print $1}')

if [[ "$h1" != "$h2" ]]; then
    echo "FAIL: transcript not stable across separate bursts ($h1 != $h2)" >&2
    exit 1
fi
echo "PASS: transcript captured, <=80 cols/line, stable across bursts ($h1)"
