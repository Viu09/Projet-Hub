#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WEB_DIR="$ROOT_DIR/web"
CRATE_NAME="snake-rust"

cd "$ROOT_DIR"

rustup target add wasm32-unknown-unknown >/dev/null

# Release build for smoother browser perf (100-bot demo)
cargo build --release --target wasm32-unknown-unknown --features demo100

WASM_IN="$ROOT_DIR/target/wasm32-unknown-unknown/release/${CRATE_NAME}.wasm"
WASM_OUT="$WEB_DIR/${CRATE_NAME}_demo100.wasm"

cp "$WASM_IN" "$WASM_OUT"

echo "WASM demo ready: $WASM_OUT"
