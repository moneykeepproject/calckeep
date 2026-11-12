#!/usr/bin/env bash
# Build the WebAssembly version into web/pkg and serve it locally.
# One-time setup:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-pack
set -euo pipefail
cd "$(dirname "$0")"
wasm-pack build --release --target web --out-dir web/pkg --no-typescript
echo
echo "Built. Serve the web/ folder, e.g.:"
echo "  python3 -m http.server 8080 --directory web"
echo "then open http://localhost:8080"
