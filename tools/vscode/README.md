# mimas for VS Code

Syntax highlighting, snippets, and language configuration for [mimas](https://mim.as) `.mim` files, plus diagnostics and hover through the mimas language server.

The server, `mimas-lsp`, comes bundled with the extension on Windows, macOS, and Linux (x64 and arm64). `mimas.serverPath` points it at a different build, like one from `cargo install mimas-lsp`. On any other platform the extension looks for `mimas-lsp` on `PATH`. You can still get highlighting without the language server.

A Rust host ran from its cargo `target` directory writes its API to `target/mimas/api.json`, which the server load to resolve host types. `mimas.apiPath` points it elsewhere, or `off` for std only. "mimas: Select Host API Manifest" sets it with a file dialog, and "mimas: Restart Language Server" restarts the server, which also happens on its own when either setting changes. `mimas.trace.server` logs the messages to the "mimas" output channel.

Dual-licensed under MIT or Apache 2.0, same as mimas.
