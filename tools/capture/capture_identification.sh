#!/usr/bin/env bash
#
# Phase 4.5.4.1 — Build identification capture harness and emit golden files.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUILD="$ROOT/build/ref"
OUT="$ROOT/tests/golden/identification"

mkdir -p "$BUILD" "$OUT"

SRCS=$(ls "$ROOT"/src/*.cpp | grep -v '/main\.cpp$')

NCURSES_LIBS=$(ncurses6-config --libs 2>/dev/null \
    || ncurses5-config --libs 2>/dev/null \
    || echo -lncurses)

# shellcheck disable=SC2086
c++ -std=c++14 -O2 -I"$ROOT/src" $SRCS \
    "$ROOT/tools/capture/identification_capture.cpp" \
    $NCURSES_LIBS \
    -o "$BUILD/identification_capture"

MAIN_SEED=12345
for SEED in 1 42 12345 1700000000; do
    "$BUILD/identification_capture" "$OUT" "$MAIN_SEED" "$SEED"
done

echo "Identification goldens written to $OUT"
