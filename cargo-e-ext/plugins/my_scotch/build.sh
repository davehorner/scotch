#!/usr/bin/env bash
# Sample build script for my_scotch plugin
set -euo pipefail

echo "Building my_scotch native dynamic library..."
cargo build

echo "\nBuilding my_scotch WebAssembly (wasm32-unknown-unknown, release)..."
# Ensure the wasm target is installed
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
cargo build --release --target wasm32-unknown-unknown

echo "\nBuild complete. Artifacts:"
echo "- Native dynamic library (host target):"
ls -lh target/debug/libmy_scotch.* 2>/dev/null || ls -lh target/debug/my_scotch.* 2>/dev/null || true
echo "- WebAssembly module (release):"
ls -lh target/wasm32-unknown-unknown/release/my_scotch.wasm || true