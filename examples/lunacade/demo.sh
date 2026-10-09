#!/usr/bin/env bash
# Builds a cart into `target/demo` and serves a page that is nothing but the game running:
#   ./demo.sh carts/pong [port]
# The wasm is built once and kept. Pass --build to rebuild it after changing lunacade's Rust.
set -euo pipefail

cd "$(dirname "$0")"

build=
if [ "${1:-}" = "--build" ]; then
    build=1
    shift
fi
cart=${1:-}
port=${2:-8000}
if [ ! -d "$cart" ]; then
    echo "usage: demo.sh [--build] <cart dir> [port]" >&2
    exit 1
fi

out=target/demo
mkdir -p "$out"

if [ -n "$build" ] || [ ! -f "$out/wasm/lunacade_bg.wasm" ]; then
    # the cli has to match the crate, or the glue it writes won't load the wasm
    version=$(grep -A1 'name = "wasm-bindgen"$' Cargo.lock | sed -n 's/version = "\(.*\)"/\1/p')
    installed=$(wasm-bindgen --version 2>/dev/null | cut -d' ' -f2 || true)
    if [ "$installed" != "$version" ]; then
        echo "wasm-bindgen $version needed, found ${installed:-nothing}:" >&2
        echo "  cargo install wasm-bindgen-cli --version $version --locked" >&2
        exit 1
    fi
    cargo build --release --target wasm32-unknown-unknown -p lunacade
    wasm-bindgen --target web --no-typescript \
        --out-dir "$out/wasm" --out-name lunacade \
        target/wasm32-unknown-unknown/release/lunacade.wasm
fi

cp demo/index.html ../../book/src/lunacade/console.js ../../book/src/lunacade/dom.js "$out"

# the files `Cart::from_dir` would read, as the json the page loads
python3 - "$cart" > "$out/cart.json" <<'PY'
import json, os, sys

root = sys.argv[1]
files = {}
for dir, dirs, names in os.walk(root):
    dirs[:] = [name for name in dirs if not name.startswith(".")]
    for name in names:
        key = os.path.relpath(os.path.join(dir, name), root).replace(os.sep, "/")
        if name.endswith(".mim") or key == "sprites.txt":
            with open(os.path.join(dir, name), encoding="utf-8") as file:
                files[key] = file.read()
json.dump({"files": files}, sys.stdout)
PY

echo "serving $cart at http://localhost:$port"
python3 -m http.server "$port" --directory "$out"
