#!/usr/bin/env bash
# Full CI-like verification: build + test + fmt.
# This is the canonical way to confirm the scaffold is healthy.
#
# Usage:
#   cd hypercolator && ./scripts/verify.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
CARGO_HOME="${CARGO_HOME:-/home/runner/workspace/.local/share/.cargo}"
export PATH="$CARGO_HOME/bin:$HOME/.local/bin:$PATH"

echo "==================================================="
echo " Hypercolator scaffold verification"
echo "==================================================="

echo ""
echo "--- Build (BPF) ---"
"$SCRIPT_DIR/build.sh"

echo ""
echo "--- Tests (native) ---"
# Run tests with the default nix-rust toolchain (no BPF target needed).
cargo test --lib --manifest-path "$REPO_ROOT/Cargo.toml" 2>&1 | tail -5

echo ""
echo "--- Formatting ---"
# NOTE: plain 'cargo fmt --check' fails in this environment because the default
# nix-rust toolchain does not include cargo-fmt.  scripts/fmt.sh uses
# 'cargo +nix-rust-full fmt' which is the correct CI entry point.
"$SCRIPT_DIR/fmt.sh"

echo ""
echo "==================================================="
echo " All checks passed."
echo "==================================================="
