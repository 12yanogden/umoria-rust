#!/usr/bin/env bash
# Build the Rust umoria binary used by golden capture / live replay.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"

cargo build --release --bin umoria --manifest-path "$ROOT/Cargo.toml"

if [[ ! -x "$ROOT/target/release/umoria" ]]; then
    echo "FAIL: $ROOT/target/release/umoria does not exist or is not executable" >&2
    exit 1
fi

echo "PASS: Rust binary built at $ROOT/target/release/umoria"
