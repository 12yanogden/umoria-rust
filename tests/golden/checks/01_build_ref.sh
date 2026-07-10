#!/usr/bin/env bash
# Phase 1.4.1 test: the reference C++ build succeeds and produces an executable
# umoria/umoria binary.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"

"$ROOT/tools/capture/build_ref.sh"

if [[ ! -x "$ROOT/umoria/umoria" ]]; then
    echo "FAIL: $ROOT/umoria/umoria does not exist or is not executable" >&2
    exit 1
fi

echo "PASS: reference binary built at $ROOT/umoria/umoria"
