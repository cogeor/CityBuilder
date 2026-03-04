#!/usr/bin/env bash
set -euo pipefail

echo "=== TownBuilder Test Suite ==="

# Rust tests
echo "--- Rust unit tests ---"
cd packages/engine-wasm
cargo test --lib 2>&1 | tail -5
echo ""

echo "--- Rust integration tests ---"
cargo test --test integration 2>&1 | tail -5
cd ../..
echo ""

# TypeScript tests
echo "--- TypeScript tests ---"
npx vitest run 2>&1 | tail -20

echo ""
echo "=== All tests complete ==="
