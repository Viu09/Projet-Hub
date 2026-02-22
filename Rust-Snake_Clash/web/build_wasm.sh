#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WEB_DIR="$ROOT_DIR/web"
CRATE_NAME="snake-rust"

cd "$ROOT_DIR"

rustup target add wasm32-unknown-unknown >/dev/null

# Release build for smoother browser perf
cargo build --release --target wasm32-unknown-unknown

WASM_IN="$ROOT_DIR/target/wasm32-unknown-unknown/release/${CRATE_NAME}.wasm"
WASM_OUT="$WEB_DIR/${CRATE_NAME}.wasm"

cp "$WASM_IN" "$WASM_OUT"
cp "$WASM_IN" "$WEB_DIR/${CRATE_NAME}_playable.wasm"

echo "WASM ready: $WASM_OUT"
