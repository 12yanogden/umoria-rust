#!/usr/bin/env bash
#
# Build the standalone RNG capture harness and emit golden files under
# tests/golden/rng/ via `cargo run --bin rng_capture`.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$ROOT/tests/golden/rng"

mkdir -p "$OUT"

cargo run --quiet --bin rng_capture --manifest-path "$ROOT/Cargo.toml" -- "$OUT"
echo "RNG goldens written to $OUT"
