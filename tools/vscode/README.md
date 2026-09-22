# mimas for VS Code

Syntax highlighting, snippets, and language configuration for [mimas](https://mim.as) `.mim` files, plus diagnostics and hover through the mimas language server.

The server is the `mimas-lsp` binary from the mimas repo (`cargo install --path crates/lsp`). The extension looks for it on `PATH`, or wherever `mimas.serverPath` points -- without it you still get highlighting. `mimas.trace.server` logs the messages to the "mimas" output channel.

The TextMate grammar's source of truth is `tools/highlighter/mimas.tmLanguage.json`; `npm run sync-grammar` copies it in (also runs automatically on `vsce package`). `npm install`, then <kbd>F5</kbd> compiles and opens an Extension Development Host.

Dual-licensed under MIT or Apache 2.0, same as mimas.
