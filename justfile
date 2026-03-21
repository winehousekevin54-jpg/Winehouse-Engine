# Winehouse Engine - Task Runner

# Build the WASM bridge crate
build-wasm:
    wasm-pack build crates/winehouse-wasm-bridge --target web --out-dir pkg

# Build WASM in release mode
build-wasm-release:
    wasm-pack build crates/winehouse-wasm-bridge --target web --release --out-dir pkg

# Start the editor dev server (builds WASM first)
dev: build-wasm
    cd packages/editor && pnpm dev

# Run all Rust tests (bridge excluded - it's WASM-only)
test-rust:
    cargo test --workspace --exclude winehouse-wasm-bridge

# Run all TypeScript tests
test-ts:
    pnpm -r test

# Run all tests
test: test-rust test-ts

# Lint Rust code
lint-rust:
    cargo clippy --workspace -- -D warnings

# Lint TypeScript code
lint-ts:
    pnpm -r lint

# Lint everything
lint: lint-rust lint-ts

# Full production build
build: build-wasm-release
    pnpm --filter @winehouse/editor build

# Install all dependencies
setup:
    pnpm install
    rustup target add wasm32-unknown-unknown

# Clean build artifacts
clean:
    cargo clean
    rm -rf crates/winehouse-wasm-bridge/pkg
    rm -rf packages/editor/dist
