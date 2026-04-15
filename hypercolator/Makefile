.PHONY: build test fmt verify clean

# Build the Hypercolator BPF program.
build:
        ./scripts/build.sh

# Run the Percolator unit tests with the native toolchain.
test:
        CARGO_HOME=$${CARGO_HOME:-/home/runner/workspace/.local/share/.cargo} \
          $${CARGO_HOME:-/home/runner/workspace/.local/share/.cargo}/bin/cargo \
          test --lib

# Check source formatting via nix-rust-full toolchain (has cargo-fmt).
# NOTE: plain 'cargo fmt --check' fails here; the default nix-rust toolchain
# does not include cargo-fmt.  Always use 'make fmt' or './scripts/fmt.sh'.
fmt:
        ./scripts/fmt.sh

# Apply source formatting in-place.
fmt-apply:
        ./scripts/fmt.sh apply

# Run build + test + fmt (CI-equivalent).
verify:
        ./scripts/verify.sh

clean:
        cargo clean
