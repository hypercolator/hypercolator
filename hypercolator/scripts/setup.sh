#!/usr/bin/env bash
# Hypercolator toolchain bootstrap
#
# Run this once in a fresh environment to install the Solana and Anchor CLIs.
# Subsequent builds use ./scripts/build.sh (or `make build`).
#
# Idempotent: skips steps where the correct version is already installed.
#
# Required versions:
#   Solana CLI  1.18.26
#   Anchor CLI  0.30.1
#   Rust        any recent stable (supplied by Replit nix module)
set -euo pipefail

SOLANA_VERSION="1.18.26"
ANCHOR_VERSION="0.30.1"

CARGO_HOME="${CARGO_HOME:-/home/runner/workspace/.local/share/.cargo}"
export PATH="$CARGO_HOME/bin:$HOME/.local/bin:$PATH"

# ---- 1. Solana CLI ----
SOLANA_BIN="$HOME/.local/share/solana/install/releases/${SOLANA_VERSION}/solana-release/bin/solana"
if "$SOLANA_BIN" --version 2>/dev/null | grep -q "$SOLANA_VERSION"; then
  echo "[skip] Solana CLI ${SOLANA_VERSION} already installed"
else
  echo "[install] Solana CLI ${SOLANA_VERSION}..."
  sh -c "$(curl -sSfL https://release.solana.com/v${SOLANA_VERSION}/install)" \
    -- --data-dir "$HOME/.local/share/solana/install"
fi

# Ensure active solana is on PATH.
SOLANA_ACTIVE="$HOME/.local/share/solana/install/active_release/bin"
if [ -d "$SOLANA_ACTIVE" ] && ! echo "$PATH" | grep -q "$SOLANA_ACTIVE"; then
  export PATH="$SOLANA_ACTIVE:$PATH"
fi
echo "Solana: $(solana --version 2>/dev/null || echo 'not in PATH yet')"

# ---- 2. Re-link solana rustup toolchain ----
PLATFORM_TOOLS_RUST="$HOME/.local/share/solana/install/releases/${SOLANA_VERSION}/solana-release/bin/sdk/sbf/dependencies/platform-tools/rust"
if [ -d "$PLATFORM_TOOLS_RUST" ]; then
  "$CARGO_HOME/bin/rustup" toolchain link solana "$PLATFORM_TOOLS_RUST" 2>/dev/null || true
  echo "Rustup 'solana' toolchain linked."
fi

# ---- 3. Anchor CLI (via avm - Anchor Version Manager) ----
AVM_BIN="$CARGO_HOME/bin/avm"
ANCHOR_BIN="$HOME/.local/bin/anchor"

if "$ANCHOR_BIN" --version 2>/dev/null | grep -q "$ANCHOR_VERSION"; then
  echo "[skip] Anchor CLI ${ANCHOR_VERSION} already installed"
else
  echo "[install] Anchor CLI ${ANCHOR_VERSION} via avm..."
  # Install avm if not present
  if [ ! -x "$AVM_BIN" ]; then
    "$CARGO_HOME/bin/cargo" install --git https://github.com/coral-xyz/anchor \
      avm --locked --force 2>&1 | tail -5
  fi
  "$AVM_BIN" install "${ANCHOR_VERSION}"
  "$AVM_BIN" use "${ANCHOR_VERSION}"
fi

if command -v anchor >/dev/null 2>&1; then
  echo "Anchor:  $(anchor --version)"
else
  echo "WARNING: anchor not in PATH. Add ~/.local/bin or avm output dir to PATH."
fi

# ---- 4. node_modules for anchor tests (optional) ----
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ -f "$REPO_ROOT/package.json" ] && ! [ -d "$REPO_ROOT/node_modules" ]; then
  echo "[install] npm dependencies..."
  npm install --prefix "$REPO_ROOT" 2>&1 | tail -3
fi

echo ""
echo "Setup complete. Run 'make build' or './scripts/build.sh' to compile."
