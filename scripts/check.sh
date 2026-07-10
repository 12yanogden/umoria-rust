#!/usr/bin/env bash
# Local quality gates — mirrors CI (fmt / clippy / test / doc).
# Usage: ./scripts/check.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "==> cargo fmt --all -- --check"
cargo fmt --all -- --check

echo "==> cargo clippy --all-targets --all-features -- -Dwarnings"
cargo clippy --all-targets --all-features -- -Dwarnings

echo "==> cargo test --all-features"
cargo test --all-features

echo "==> cargo doc --no-deps --all-features (-D warnings)"
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

if command -v cargo-deny >/dev/null 2>&1; then
    echo "==> cargo deny check"
    cargo deny check
elif [[ -x "${HOME}/.cargo/bin/cargo-deny" ]]; then
    echo "==> cargo deny check"
    "${HOME}/.cargo/bin/cargo-deny" check
else
    echo "note: cargo-deny not installed; skipping (CI runs EmbarkStudios/cargo-deny-action)"
fi

echo "All checks passed."
