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

  # Step 3b: Optimize WASM binary (optional, requires wasm-opt)
  if command -v wasm-opt &> /dev/null; then
    echo "--- WASM optimization ---"
    wasm-opt -O3 --enable-simd packages/engine-wasm/pkg/townbuilder_engine_bg.wasm -o packages/engine-wasm/pkg/townbuilder_engine_bg.wasm
    echo "WASM binary optimized"
  else
    echo "--- Skipping WASM optimization (wasm-opt not found) ---"
  fi

  # Step 3c: Check WASM binary size budget
  WASM_FILE="packages/engine-wasm/pkg/townbuilder_engine_bg.wasm"
  if [ -f "$WASM_FILE" ]; then
    SIZE=$(stat -c%s "$WASM_FILE" 2>/dev/null || stat -f%z "$WASM_FILE")
    echo "WASM binary size: $SIZE bytes"
    MAX_SIZE=2097152  # 2MB budget
    if [ "$SIZE" -gt "$MAX_SIZE" ]; then
      echo "WARNING: WASM binary ($SIZE bytes) exceeds budget ($MAX_SIZE bytes)"
    fi
  fi
else
  echo "--- Skipping WASM build (wasm-pack not found) ---"
fi

# Step 4: Web build
echo "--- Web build ---"
pnpm run build

echo "=== Build complete ==="
