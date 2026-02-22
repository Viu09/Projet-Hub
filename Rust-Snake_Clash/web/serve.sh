#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR"

# Allow overriding the port: `PORT=4001 bash web/serve.sh`
PORT="${PORT:-4000}"

port_in_use() {
  local p="$1"
  if command -v fuser >/dev/null 2>&1; then
    fuser "${p}/tcp" >/dev/null 2>&1
    return $?
  fi
  if command -v ss >/dev/null 2>&1; then
    ss -ltn | awk '{print $4}' | grep -qE "(^|:)${p}$"
    return $?
  fi
  return 1
}

if port_in_use "$PORT"; then
  echo "Port ${PORT} already in use. Picking another port..." >&2
  for candidate in $(seq $((PORT + 1)) $((PORT + 50))); do
    if ! port_in_use "$candidate"; then
      PORT="$candidate"
      break
    fi
  done
fi

if port_in_use "$PORT"; then
  echo "No free port found near ${PORT}." >&2
  echo "Tip: stop whoever is using it: fuser -k 4000/tcp" >&2
  exit 1
fi

if ! command -v basic-http-server >/dev/null 2>&1; then
  echo "basic-http-server not found. Install with: cargo install basic-http-server"
  exit 1
fi

echo "Serving on http://127.0.0.1:${PORT}/web/ (Ctrl+C to stop)"
basic-http-server . -a "127.0.0.1:${PORT}"
