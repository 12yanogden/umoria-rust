#!/usr/bin/env bash
#
# Run every golden-harness check (01..08) in order.
#
# Each check drives the Rust umoria binary and/or the pty capture tooling.
# Pass --regen to have check 08 do a full rebuild + re-capture idempotence pass.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CHECKS="$ROOT/tests/golden/checks"

rc=0
for c in "$CHECKS"/[0-9][0-9]_*.sh; do
    name="$(basename "$c")"
    echo "==> $name"
    if [[ "$name" == 08_* && "${1:-}" == "--regen" ]]; then
        "$c" --regen || { echo "FAILED: $name"; rc=1; }
    else
        "$c" || { echo "FAILED: $name"; rc=1; }
    fi
done

if [[ $rc -eq 0 ]]; then
    echo "ALL GOLDEN CHECKS PASSED"
else
    echo "SOME GOLDEN CHECKS FAILED" >&2
fi
exit $rc
