#!/usr/bin/env bash
set -euo pipefail

echo "=== TownBuilder Build ==="

# Step 1: Rust tests
echo "--- Rust tests ---"
cd packages/engine-wasm
cargo test --lib
cargo test --test integration
cd ../..

# Step 2: TypeScript tests
echo "--- TypeScript tests ---"
pnpm run test

# Step 3: WASM build (optional, requires wasm-pack)
if command -v wasm-pack &> /dev/null; then
  echo "--- WASM build ---"
  cd packages/engine-wasm
  wasm-pack build --target web --out-dir pkg
  cd ../..
else
  echo "--- Skipping WASM build (wasm-pack not found) ---"
fi

# Step 4: Web build
echo "--- Web build ---"
pnpm run build

echo "=== Build complete ==="
