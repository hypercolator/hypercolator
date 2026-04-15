#!/usr/bin/env bash
# Hypercolator BPF build script
#
# Wraps anchor build with the environment needed by the Solana BPF toolchain
# (platform-tools v1.41, Rust 1.75).  Use `make build` or `./scripts/build.sh`
# instead of calling `anchor build` directly.
#
# Key constraints (full explanation in docs/build-toolchain.md):
#   CARGO env var - rustup proxy so cargo-build-sbf uses cargo 1.88 for
#     dependency resolution (correct cfg() for sbf-solana-solana) while still
#     invoking the Solana platform-tools rustc for actual BPF compilation.
#   Cargo.lock v3 - required by cargo-build-sbf's internal cargo 1.75 parser.
#   --no-idl - anchor-syn 0.30.1 calls proc_macro2::Span::source_file (unstable
#     API absent on platform-tools rustc 1.75); IDL is embedded by anchor-lang.
#   vendor/wasm-bindgen - local patch restricting panic_any to target_arch=wasm32;
#     the sbf-solana-solana std does not expose std::panic::panic_any.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Persistent cargo home (workspace storage - survives container restarts).
CARGO_HOME="${CARGO_HOME:-/home/runner/workspace/.local/share/.cargo}"

# The rustup proxy cargo - the one that understands `+toolchain` syntax.
CARGO_BIN="$CARGO_HOME/bin/cargo"
RUSTUP_BIN="$CARGO_HOME/bin/rustup"

if [ ! -x "$CARGO_BIN" ]; then
  echo "ERROR: cargo not found at $CARGO_BIN"
  echo "Ensure CARGO_HOME points to the workspace-persistent cargo installation."
  exit 1
fi

# Put rustup-proxy cargo and anchor on PATH so they are found by anchor internally.
export PATH="$CARGO_HOME/bin:$HOME/.local/bin:$PATH"

# ---- Re-link the solana toolchain (RUSTUP_HOME is session-ephemeral) ----
# The Solana platform-tools rustc is at a stable path in ~/.local/share.
PLATFORM_TOOLS_RUST="${SOLANA_INSTALL_DIR:-$HOME/.local/share/solana/install/releases/1.18.26/solana-release}/bin/sdk/sbf/dependencies/platform-tools/rust"
if [ -d "$PLATFORM_TOOLS_RUST" ]; then
  "$RUSTUP_BIN" toolchain link solana "$PLATFORM_TOOLS_RUST" 2>/dev/null || true
else
  echo "ERROR: Solana platform-tools not found at: $PLATFORM_TOOLS_RUST"
  echo "Run: sh -c \"\$(curl -sSfL https://release.solana.com/v1.18.26/install)\""
  exit 1
fi
# Verify the toolchain link is usable.
"$CARGO_BIN" +solana --version >/dev/null 2>&1 || {
  echo "ERROR: 'cargo +solana' is not working after toolchain link."
  echo "  RUSTUP_HOME: ${RUSTUP_HOME:-~/.rustup}"
  echo "  PLATFORM_TOOLS_RUST: $PLATFORM_TOOLS_RUST"
  exit 1
}

# ---- Ensure Cargo.lock is v3 (cargo-build-sbf's internal cargo 1.75 format) ----
LOCK="$REPO_ROOT/Cargo.lock"
if [ ! -f "$LOCK" ] || head -3 "$LOCK" | grep -q "version = 4"; then
  echo "Re-generating Cargo.lock as v3..."
  "$CARGO_BIN" +solana generate-lockfile --manifest-path "$REPO_ROOT/Cargo.toml"
fi

# ---- Build ----
cd "$REPO_ROOT"
echo "=== anchor build --no-idl ==="
# Set CARGO to the rustup proxy so anchor's internal cargo-build-sbf call uses
# the proxy, which correctly dispatches `+solana` to the linked toolchain.
CARGO="$CARGO_BIN" anchor build --no-idl "$@"

echo ""
echo "Build OK.  Output: $REPO_ROOT/target/deploy/hypercolator.so"
