#!/usr/bin/env bash
# Manifest integrity.
#
# tests/golden/manifest.json must be valid JSON, list an entry for every golden
# under tests/golden/{rng,save,scores,transcripts} (excluding checks/, the
# manifest itself, and volatile *.raw intermediates), and every recorded sha256
# must match the on-disk file. For clock-volatile save/score goldens the hash is
# taken over the decoded, masked plaintext (hash_method sha256-masked-*), so it
# stays constant across regen; golden_manifest.py verify enforces all of this.
#
# With --regen this additionally runs tools/capture/regen.sh (full rebuild +
# re-capture, needs required_permissions:["all"]) and re-verifies, proving
# regen.sh reproduces all goldens with unchanged hashes.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
MANIFEST="$ROOT/tests/golden/manifest.json"

[[ -f "$MANIFEST" ]] || { echo "FAIL: $MANIFEST missing" >&2; exit 1; }

# Valid JSON?
python3 -c "import json,sys; json.load(open('$MANIFEST'))" \
 || { echo "FAIL: manifest.json is not valid JSON" >&2; exit 1; }

if [[ "${1:-}" == "--regen" ]]; then
 echo "running full regen (rebuild + re-capture)..."
 "$ROOT/tools/capture/regen.sh"
fi

if ! python3 "$ROOT/tools/capture/golden_manifest.py" verify; then
 echo "FAIL: manifest does not match on-disk goldens" >&2
 exit 1
fi

echo "PASS: manifest.json valid and consistent with on-disk goldens"
