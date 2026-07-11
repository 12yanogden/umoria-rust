#!/usr/bin/env bash
#
# Recorded-input playthrough wrapper.
#
# Usage: play.sh <name> <seed>
# Reads tests/golden/transcripts/<name>.keys (raw keystroke bytes)
# Writes tests/golden/transcripts/<name>.screen (escape-stripped, stable)
# tests/golden/transcripts/<name>.raw (raw pty output)
# Copies the resulting game.sav -> tests/golden/save/<name>.sav (if produced)
#
# Determinism: always passes -s <seed>, fixes TERM=xterm/LINES=24/COLS=80 (set by
# pty_driver.py), and the keystroke scripts end in an explicit ^X save+quit.
# Runs the Rust `umoria` binary from the repository root (data/, LICENSE, …).
set -euo pipefail

NAME="${1:?usage: play.sh <name> <seed>}"
SEED="${2:?usage: play.sh <name> <seed>}"

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="${UMORIA_BIN:-$ROOT/target/release/umoria}"
RUNDIR="$ROOT"
TDIR="$ROOT/tests/golden/transcripts"
SDIR="$ROOT/tests/golden/save"
KEYS="$TDIR/$NAME.keys"

if [[ ! -x "$BIN" ]]; then
    cargo build --release --bin umoria --manifest-path "$ROOT/Cargo.toml"
fi
[[ -x "$BIN" ]] || { echo "play.sh: missing binary $BIN" >&2; exit 1; }
[[ -f "$KEYS" ]] || { echo "play.sh: missing keystroke script $KEYS" >&2; exit 1; }
mkdir -p "$TDIR" "$SDIR"

# Fresh save/score state in the run dir; copied out afterward if created.
rm -f "$RUNDIR/game.sav"
cp "$ROOT/tests/golden/scores/scores_initial.dat" "$RUNDIR/scores.dat"

python3 "$ROOT/tools/capture/pty_driver.py" \
    --binary "$BIN" \
    --seed "$SEED" \
    --save game.sav \
    --keys "$KEYS" \
    --raw "$TDIR/$NAME.raw" \
    --screen "$TDIR/$NAME.screen" \
    --cwd "$RUNDIR" \
    --char-delay 0.15 \
    --timeout 30

if [[ -f "$RUNDIR/game.sav" ]]; then
    cp "$RUNDIR/game.sav" "$SDIR/$NAME.sav"
fi

echo "play.sh: captured $TDIR/$NAME.screen"
