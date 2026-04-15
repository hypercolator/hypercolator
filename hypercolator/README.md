# Hypercolator

Permissionless perpetual futures DEX on Solana, powered by the Percolator
risk engine by Anatoly Yakovenko.

## Quick start (fresh environment)

```bash
# 1. Install Solana CLI 1.18.26 and Anchor CLI 0.30.1
./scripts/setup.sh

# 2. Build the BPF program
make build          # or: ./scripts/build.sh

# 3. Run unit tests
make test           # or: cargo test --lib

# 4. Check formatting
make fmt            # or: ./scripts/fmt.sh

# 5. Full CI-equivalent check
make verify         # or: ./scripts/verify.sh
```

## Project layout

```
programs/hypercolator/   Anchor program (entry point)
crates/percolator/       Percolator risk engine (vendored, no_std)
vendor/wasm-bindgen/     wasm-bindgen 0.2.117 with SBF patch (see below)
scripts/
  setup.sh              Installs Solana CLI + Anchor CLI (run once)
  build.sh              Builds the BPF .so via anchor build --no-idl
  fmt.sh                cargo fmt using nix-rust-full (has rustfmt)
  verify.sh             Full CI check: build + test + fmt
Makefile                Convenience targets: build / test / fmt / verify
```

## Required toolchain

| Tool         | Version  | Install                                |
|-------------|----------|----------------------------------------|
| Solana CLI  | 1.18.26  | `./scripts/setup.sh` (or see below)   |
| Anchor CLI  | 0.30.1   | `./scripts/setup.sh`                  |
| Rust        | stable   | provided by Replit nix `rust-stable` module |
| rustfmt     | -        | provided by nix store (auto-detected) |

To install Solana CLI manually:
```bash
sh -c "$(curl -sSfL https://release.solana.com/v1.18.26/install)"
```

To install Anchor CLI manually (via avm):
```bash
cargo install --git https://github.com/coral-xyz/anchor avm --locked
avm install 0.30.1 && avm use 0.30.1
```

## Build notes

### Why `./scripts/build.sh` instead of `anchor build`?

The Solana BPF toolchain has three environment constraints:

1. **`CARGO` env var** - must point to the rustup proxy (`$CARGO_HOME/bin/cargo`)
   so that `cargo-build-sbf` dispatches `+solana` to the linked toolchain.
   Without this, plain `anchor build` fails with "no such command: +solana".

2. **Cargo.lock v3** - `cargo-build-sbf` ships an internal cargo 1.75 that
   cannot parse Cargo.lock v4. `scripts/build.sh` auto-regenerates the lock
   using `cargo +solana generate-lockfile`.

3. **`--no-idl`** - anchor-syn 0.30.1 calls `proc_macro2::Span::source_file`
   which is an unstable API absent on the Solana platform-tools rustc 1.75.

### vendor/wasm-bindgen

wasm-bindgen 0.2.117's `js_panic()` calls `std::panic::panic_any` which is
absent from the Solana SBF standard library. The local patch in
`vendor/wasm-bindgen/` adds a `target_arch = "wasm32"` guard so the SBF
target falls back to `core::panic!` instead.

## Running on devnet

```bash
solana-keygen new -o ~/.config/solana/id.json   # create wallet
solana airdrop 2 --url devnet                   # fund it
make build
anchor deploy --provider.cluster devnet
```
