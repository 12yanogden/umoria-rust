#!/usr/bin/env bash
#
# Regenerate ALL golden artifacts and refresh the manifest.
#
# Rebuilds the Rust binary, re-captures every golden (RNG, playthrough transcript
# + save, score file + -d screen), and rewrites tests/golden/manifest.json.
#
# Clock-volatile save/score goldens will have different raw bytes each run (the
# timestamp / date_of_birth / birth_date fields), but the manifest hashes those
# entries over the decoded, masked plaintext, so `golden_manifest.py verify`
# stays green across runs.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CAP="$ROOT/tools/capture"
BIN="$ROOT/target/release/umoria"

echo "==> [1/6] build Rust binary"
cargo build --release --bin umoria --manifest-path "$ROOT/Cargo.toml"

echo "==> [2/6] capture RNG goldens"
"$CAP/capture_rng.sh"

echo "==> [3/6] refresh pristine score file golden"
cp "$ROOT/data/scores.dat" "$ROOT/tests/golden/scores/scores_initial.dat"

echo "==> [4/6] capture playthrough transcript + save (newchar_seed42, -s 42)"
UMORIA_BIN="$BIN" "$CAP/play.sh" newchar_seed42 42

echo "==> [5/6] capture high-score screen (umoria -s 42 -d)"
python3 "$CAP/pty_driver.py" \
    --binary "$BIN" \
    --seed 42 --extra-arg=-d \
    --keys "$ROOT/tests/golden/transcripts/scores_screen.keys" \
    --raw "$ROOT/tests/golden/scores/.scores_screen.raw" \
    --screen "$ROOT/tests/golden/scores/scores_screen.txt" \
    --cwd "$ROOT" \
    --char-delay 0.2 \
    --timeout 15

echo "==> [6/6] rewrite manifest.json"
python3 "$CAP/golden_manifest.py" write

echo "regen complete."
