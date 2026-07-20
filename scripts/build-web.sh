#!/usr/bin/env bash
# Build the SQL Lens web UI into crates/sql-lens-app/web/dist for single-process serving.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WEB_DIR="${ROOT_DIR}/crates/sql-lens-app/web"

cd "${WEB_DIR}"
if [[ ! -d node_modules ]]; then
  npm install
fi
npm run build

echo "Built web UI at ${WEB_DIR}/dist"
echo "Point web.static_dir at that path, or leave it unset to auto-discover from the repo root."
