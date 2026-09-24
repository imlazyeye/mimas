# mimas for VS Code

Language support for [mimas](https://mim.as) `.mim` files in Visual Studio Code.

## Currently supported

- Text highlighting
- Inline diagnostics
- Type information and documentation on hover
- Go to definition
- Rename
- Find references
- Outline view
- Inlay hints
- Snippets

## Language server

The server, `mimas-lsp`, comes bundled with the extension on Windows, macOS, and Linux (x64 and arm64). `mimas.serverPath` points it at a different build, like one from `cargo install mimas-lsp`. On any other platform the extension looks for `mimas-lsp` on `PATH`. You can still get highlighting without the language server.

## Getting host types

A Rust host run from its cargo `target` directory writes its API to `target/mimas/api.json`, which the server loads to resolve host types. `mimas.apiPath` points it elsewhere, or `off` for std only. "mimas: Select Host API Manifest" sets it with a file dialog, and "mimas: Restart Language Server" restarts the server, which also happens on its own when either setting changes. `mimas.trace.server` logs the messages to the "mimas" output channel.
