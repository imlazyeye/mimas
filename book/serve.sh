#!/usr/bin/env bash
# Builds the book's wasm demos, then serves the book with live reload.
set -euo pipefail

cd "$(dirname "$0")/.."

book/wasm.sh
mdbook serve book --port "${1:-8000}"
