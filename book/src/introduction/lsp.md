# Language Server

mimas ships a language server to use in the editor of your choice. It is still very young but offers
diagnostics, hover, navigation, rename, outlines, and inlay hints for scripting use. Scripts that
use a Rust host's functions and types are checked against that host's API once it has run -- see
[Host APIs](#host-apis).

## What it supports

### Diagnostics

Syntax and type errors are all reported inline in your editor. The lexer and parser are resilient to
errors and will offer a full report of any syntax errors your code contains. The same is _not_ true
of today's solver, which halts the first time it encounters an error. This means that you can get
multiple syntax errors at once, but can only get one type error at a time (the same is true of the
compiler however you use it). This will be improved in future releases.

### Hover

Hovering a name shows what it is and what type it has. If its declaration has a `///` doc comment,
that shows underneath. For a function or type a Rust host registers, that's the `///` on the Rust
item.

| You hover | You get |
| --- | --- |
| a local, parameter, or field | `x: int` |
| a function or method | the owner on one line, the declaration on the next |
| a struct or enum | its qualified name, like `one::two::Foo` |
| a constant | `const SCALE: int` |
| a module | ``module `fs` `` |
| anything else | the expression's type |

Functions carry where they came from, which matters once a project has modules:

```mimas
// hovering `bar` anywhere it is used or declared
one::two::Foo
fn bar(self, fizz: int) -> Self
```

### Go to definition

Any name that resolves to a declaration in your project jumps to it, across files. That covers
locals, parameters, struct fields, functions, methods, structs, enums, and constants.

### Find references

The same names work in reverse: ask for a declaration's references and you'll get every use of it
in the project, wherever it lives. Resting your cursor on a name also highlights its other uses in
the file you're reading.

### Rename

Renaming rewrites a name everywhere it appears in the project. Renaming a member of a
[pact](../reference/pacts.md) carries every implementation with it, so the two can't drift apart.

### Outline

Your editor's outline, breadcrumbs, and symbol search list what a file declares -- types with their
fields and variants, `impl` blocks with their methods, and a script's top-level bindings. This is
the one feature that only needs the syntax tree, so it keeps working while the rest of the file
doesn't.

### Inlay hints

A `let` written without an annotation shows the type that was inferred for it.

```mimas
let count = items.len(); // your editor can render this as `let count: int = ...`
```

### Projects

Each file is checked as part of the project it sits in: the highest folder holding a `.mim` file in
the cargo package or repository around it, along with every folder below. That's what a host
loading its whole scripts folder sees, so scripts in subfolders can use every module in the project.
A project never takes in another cargo package or a `target` directory. A file outside any package
or repository is checked with the `.mim` files next to it.

## What it does not support

- **The solver is not resilient.** This means that any error will cut off type information and
  you'll be left with only that error in the editor. Everything but diagnostics and the outline
  goes quiet until the project checks cleanly again, which you'll notice most while mid-keystroke.
- **No completion, formatting, signature help, or code actions.** These are the obvious next
  steps, but none of them exist today.
- **Navigation stops at the language boundary.** Natives from the standard library or a host,
  builtin types like `int`, and module names have no mimas source to jump to, so nothing happens.
  We will support navigating to the Rust definition in the future.
- **A host's scripts have to sit in its cargo package.** Scripts outside every package, like ones
  at the root of a virtual workspace, get the standard library alone unless `mimas.apiPath` points
  the server at a manifest (which then covers every script).

## Setting it up

### The server

The VS Code extension bundles the server, so skip this step if that's your editor. Otherwise,
install the lsp via cargo:

```sh
cargo install mimas-lsp
```

That puts a `mimas-lsp` binary on your `PATH`, which is where editors will look for it by default.

### The extension

Install [mimas](https://marketplace.visualstudio.com/items?itemName=imlazyeye.mimas) from the Visual
Studio Code marketplace. Each release also attaches the extension's `.vsix` files to its
[GitHub release](https://github.com/imlazyeye/mimas/releases), which you can install by opening
the command palette and selecting "Install from VSIX". Its source lives in `tools/vscode`.

### Other editors

Any LSP client works. The server speaks the protocol over stdio, takes no arguments, and wants to
be started for the `mimas` language on `.mim` files. In Neovim, for example:

```lua
vim.lsp.config.mimas = {
    cmd = { "mimas-lsp" },
    filetypes = { "mimas" },
    root_markers = { "Cargo.toml", ".git" },
}
vim.lsp.enable("mimas")
```

### Host APIs

When a Rust host embeds mimas, its scripts use the functions, types, and constants it registers.
The server learns about those from a file the host writes. Running the host from its cargo target
dir writes its API to `target/mimas/<binary>.json`, named after the binary so hosts sharing a target
dir (i.e.: a workspace) keep their own. A script is checked against the host of the cargo package it sits in, and the
server asks `cargo` for that package's binaries and target dir. When several of them have written a
file, the newest wins. Until one has, a warning at the top of every file in the project says so.
After you change the host's API, run it again, and diagnostics pick up the change on your next edit
in that project.

- The host and the server have to be the same version of mimas. On a mismatch, the server shows a
  warning and falls back to the standard library alone.
- To turn the automatic write off, depend on `mimas` with `default-features = false`.
- Only a binary running from inside a cargo target dir writes the file automatically, so a shipped
  build never does, and neither do test or bench builds. If you want to export it yourself (e.g.
  for modding support), call `mimas::write_api`.

```rust
let library = mimas::Vm::new().install_library(mimas::library::std);
mimas::write_api(&library, "scripts/api.json".as_ref())?;
```

In VS Code, the `mimas.apiPath` setting points the server at one file for every script (relative to
the workspace folder), or turns it off with `off`, which leaves only the standard library. The
**mimas: Select Host API Manifest** command sets it with a file dialog, and **mimas: Restart
Language Server** restarts the server, which also happens on its own when the setting changes.
Other editors pass the same value as `apiPath` in the server's initialization options, where a
relative path starts from the folder the editor starts the server in. In Neovim, add it to the
config above:

```lua
init_options = { apiPath = "path/to/api.json" },
```
