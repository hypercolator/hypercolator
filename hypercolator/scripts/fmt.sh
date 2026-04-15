#!/usr/bin/env bash
# Run cargo fmt using the nix-rust-full toolchain (which has rustfmt).
# The default nix-rust toolchain is stripped and lacks rustfmt.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_HOME="${CARGO_HOME:-/home/runner/workspace/.local/share/.cargo}"
export PATH="$CARGO_HOME/bin:$HOME/.local/bin:$PATH"

# Find rustfmt from PATH (nix store supplies it even in a fresh container).
NIX_FULL_BIN="$(dirname "$(which rustfmt 2>/dev/null || true)")"
if [ -n "$NIX_FULL_BIN" ] && [ -f "$NIX_FULL_BIN/cargo-fmt" ]; then
  NIX_FULL_ROOT="$(dirname "$NIX_FULL_BIN")"
  rustup toolchain link nix-rust-full "$NIX_FULL_ROOT" 2>/dev/null || true
fi

if ! rustup toolchain list 2>/dev/null | grep -q "^nix-rust-full"; then
  echo "ERROR: nix-rust-full toolchain not found and could not be linked."
  echo "Ensure rustfmt and cargo-fmt are in PATH (they ship with the nix rust package)."
  exit 1
fi

cd "$REPO_ROOT"
echo "=== cargo fmt --check (nix-rust-full) ==="
# Pass --check if not overridden by caller args.
if [ "${1:-}" = "apply" ]; then
  cargo +nix-rust-full fmt --all
  echo "Formatting applied."
else
  cargo +nix-rust-full fmt --all --check
  echo "Format OK."
fi
