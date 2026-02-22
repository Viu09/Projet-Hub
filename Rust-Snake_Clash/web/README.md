# Web build (WASM)

## Build

From the project root:

- `bash web/build_wasm.sh`

This creates `web/snake-rust.wasm`.

## Serve locally

- Install the static server: `cargo install basic-http-server`
- Run: `bash web/serve.sh`
- Open: `http://127.0.0.1:4000/`

## Mobile performance check

- Open the same URL on your phone (same Wi‑Fi).
- Use the in-game `FPS: N` counter (top-left) to verify stability.
