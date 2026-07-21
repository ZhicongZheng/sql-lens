#!/usr/bin/env bash
# Build a single-file sql-lens binary with the web UI embedded.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

echo "==> Building web UI"
./scripts/build-web.sh

echo "==> Building release binary (embedded-ui)"
cargo build -p sql-lens-app --release

BIN="${ROOT_DIR}/target/release/sql-lens"
if [[ ! -x "${BIN}" ]]; then
  echo "error: expected binary at ${BIN}" >&2
  exit 1
fi

echo
echo "Release binary ready:"
echo "  ${BIN}"
ls -lh "${BIN}"
echo
echo "Run (from any directory; no web/dist needed):"
echo "  ${BIN} --config /path/to/sql-lens.toml"
echo
echo "Optional: set web.static_dir to override the embedded UI with on-disk assets."
