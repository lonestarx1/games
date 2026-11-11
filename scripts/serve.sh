#!/usr/bin/env bash
# Serves the site locally with caching off. WebAssembly needs http://, not file://.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PORT="${PORT:-8080}"
echo "Serving on http://localhost:$PORT"
cd "$ROOT/web" && python3 "$ROOT/scripts/dev_server.py" "$PORT"
