#!/usr/bin/env bash
#
# Phase 1.4.1 — Reproducible reference C++ build.
#
# Performs a pinned, out-of-source CMake Release (-O2) build of the reference
# Umoria binary from the unmodified C++ sources in src/. The binary and its
# resources land in $ROOT/umoria/ (per CMakeLists.txt EXECUTABLE_OUTPUT_PATH).
#
# ncurses prerequisite (linked via find_package(Curses REQUIRED)):
#   - macOS:        system SDK ncurses (Xcode/clang) — no Homebrew ncurses needed
#   - Debian/Ubuntu: libncurses-dev
#   - Fedora/RHEL:   ncurses-devel
# Requires CMake >= 3.6.
#
# NOTE: this script only drives the existing CMakeLists.txt; it modifies NOTHING
# under src/ and does not touch the game build target.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUILD="$ROOT/build/ref"   # out-of-source (in-source builds are forbidden by CMake guard)

cmake -S "$ROOT" -B "$BUILD" -DCMAKE_BUILD_TYPE=Release
cmake --build "$BUILD" --parallel

# CMake's EXECUTABLE_OUTPUT_PATH is "umoria" (relative to the build dir), so an
# out-of-source build lands the binary + resources at $BUILD/umoria/. Sync that
# tree up to $ROOT/umoria/ so the golden-capture leaves have a stable canonical
# path ($ROOT/umoria/umoria). $ROOT/umoria/ is gitignored build output.
rm -rf "$ROOT/umoria"
cp -R "$BUILD/umoria" "$ROOT/umoria"

# Record toolchain versions for reproducibility (ingested later by phase_1.4.5).
mkdir -p "$BUILD"
{
    echo "cmake: $(cmake --version | head -1)"
    echo "compiler: $(c++ --version | head -1)"
    echo "os: $(uname -a)"
} > "$BUILD/build_meta.txt"

# Sanity: the reference binary must exist and be executable.
test -x "$ROOT/umoria/umoria"
echo "Reference binary built: $ROOT/umoria/umoria"
