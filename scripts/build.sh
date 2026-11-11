#!/usr/bin/env bash
# Builds every game crate in crates/games/* to web/pkg/<name>/.
# Usage: scripts/build.sh [--dev]
set -euo pipefail
cd "$(dirname "$0")/.."
MODE="--release"
[[ "${1:-}" == "--dev" ]] && MODE="--dev"

for dir in crates/games/*/; do
  name="$(basename "$dir")"
  echo "==> building $name"
  wasm-pack build "$dir" --target web --no-typescript $MODE \
    --out-dir "$(pwd)/web/pkg/$name" --out-name "$name"
  rm -f "web/pkg/$name/package.json" "web/pkg/$name/.gitignore" "web/pkg/$name/README.md"
done
echo "Done. Run scripts/serve.sh and open http://localhost:8080"
