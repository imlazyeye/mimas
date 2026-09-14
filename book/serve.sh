#!/usr/bin/env bash
# Builds the book with the breakout demo in it, then serves it. `mdbook build` empties `book/book`,
# so the wasm has to land after it, which is why this isn't `mdbook serve`.
set -euo pipefail

cd "$(dirname "$0")/.."

port="${1:-8000}"
out="book/book/extension/breakout"
example="examples/bevy"

mdbook build book

# the cli has to match the crate, or the glue it writes won't load the wasm
version=$(grep -A1 'name = "wasm-bindgen"$' "$example/Cargo.lock" | sed -n 's/version = "\(.*\)"/\1/p')
installed=$(wasm-bindgen --version 2>/dev/null | cut -d' ' -f2 || true)
if [ "$installed" != "$version" ]; then
    echo "wasm-bindgen $version needed, found ${installed:-nothing}:" >&2
    echo "  cargo install wasm-bindgen-cli --version $version --locked" >&2
    exit 1
fi

cargo build --release --target wasm32-unknown-unknown --manifest-path "$example/Cargo.toml"
wasm-bindgen --target web --no-typescript \
    --out-dir "$out" --out-name breakout \
    "$example/target/wasm32-unknown-unknown/release/bevy.wasm"
cp -r "$example/assets" "$out/assets"

# optional, but it roughly halves the download
if command -v wasm-opt >/dev/null; then
    wasm-opt -Oz "$out/breakout_bg.wasm" -o "$out/breakout_bg.wasm"
fi

echo "serving http://localhost:$port/extension/bevy.html"
python3 -m http.server -d book/book "$port"
