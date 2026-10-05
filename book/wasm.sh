#!/usr/bin/env bash
# Builds the breakout demo and the web repl into `book/src`, where mdBook copies them into the book
# like any other file.
set -euo pipefail

cd "$(dirname "$0")/.."

# the cli has to match the crate, or the glue it writes won't load the wasm
version=$(grep -A1 'name = "wasm-bindgen"$' examples/bevy/Cargo.lock | sed -n 's/version = "\(.*\)"/\1/p')
installed=$(wasm-bindgen --version 2>/dev/null | cut -d' ' -f2 || true)
if [ "$installed" != "$version" ]; then
    echo "wasm-bindgen $version needed, found ${installed:-nothing}:" >&2
    echo "  cargo install wasm-bindgen-cli --version $version --locked" >&2
    exit 1
fi

cargo build --release --target wasm32-unknown-unknown --manifest-path examples/bevy/Cargo.toml
wasm-bindgen --target web --no-typescript \
    --out-dir book/src/extension/breakout --out-name breakout \
    examples/bevy/target/wasm32-unknown-unknown/release/bevy.wasm
cp -r examples/bevy/assets book/src/extension/breakout

cargo build --release --target wasm32-unknown-unknown --manifest-path tools/book/repl/Cargo.toml
wasm-bindgen --target web --no-typescript \
    --out-dir book/src/home --out-name repl \
    tools/book/repl/target/wasm32-unknown-unknown/release/web_repl.wasm
