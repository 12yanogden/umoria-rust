#!/usr/bin/env bash
#
# Phase 1.4.2 — Build the standalone RNG capture harness and emit golden files.
#
# Compiles every src/*.cpp EXCEPT main.cpp (to avoid a second main()) together
# with tools/capture/rng_capture.cpp, links system ncurses (needed to satisfy
# symbols pulled in from ui_io.o etc.; curses is NEVER initialized), then runs
# the harness to (re)generate tests/golden/rng/*.
#
# Modifies NOTHING under src/. Additive capture build only.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUILD="$ROOT/build/ref"
OUT="$ROOT/tests/golden/rng"

mkdir -p "$BUILD" "$OUT"

# All reference translation units except main.cpp.
SRCS=$(ls "$ROOT"/src/*.cpp | grep -v '/main\.cpp$')

# ncurses link flags: prefer a *-config helper, else fall back to -lncurses.
NCURSES_LIBS=$(ncurses6-config --libs 2>/dev/null \
    || ncurses5-config --libs 2>/dev/null \
    || echo -lncurses)

# shellcheck disable=SC2086
c++ -std=c++14 -O2 -I"$ROOT/src" $SRCS \
    "$ROOT/tools/capture/rng_capture.cpp" \
    $NCURSES_LIBS \
    -o "$BUILD/rng_capture"

"$BUILD/rng_capture" "$OUT"
echo "RNG goldens written to $OUT"
