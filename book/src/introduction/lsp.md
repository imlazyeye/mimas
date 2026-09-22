# Language Server

mimas ships a language server to use in the editor of your choice. It is still very young but offers
diagnostics, hover, navigation, rename, outlines, and inlay hints.

## What it supports

### Diagnostics

Syntax and type errors are all reported inline in your editor. The lexer and parser are resilient to
errors and will offer a full report of any syntax errors your code contains. The same is _not_ true
of today's solver, which halts the first time it encounters an error. This means that you can get
multiple syntax errors at once, but can only get one type error at a time (the same is true of the
compiler however you use it). This will be improved in future releases.

### Hover

Hovering a name shows what it is and what type it has. If its declaration has a `///` doc comment,
that shows underneath.

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

## What it does not support

- **The solver is not resilient.** This means that any error will cut off type information and
  you'll be left with only that error in the editor. Everything but diagnostics and the outline
  goes quiet until the project checks cleanly again, which you'll notice most while mid-keystroke.
- **No completion, formatting, signature help, or code actions.** These are the obvious next
  steps, but none of them exist today.
- **Navigation stops at the language boundary.** Natives from the standard library, builtin types
  like `int`, and module names have no mimas source to jump to, so nothing happens. This will come
  in the future.
- **One project is one directory.** The server walks up from the file you opened to the nearest
  directory holding a `main.mim` and treats every `.mim` beneath it as the project. A file with no
  `main.mim` above it is a project of one.

## Setting it up

### The server

Install the lsp via cargo:

```sh
cargo install mimas-lsp
```

That puts a `mimas-lsp` binary on your `PATH`, which is where editors will look for it by default.

### The extension

An extension for Visual Studio Code is housed in `tools/vscode`. An up to date .vsix is vendored
there. Inside of VSC you can install it by opening the command palette and selecting "Install from
VSIX".

### Other editors

Any LSP client works. The server speaks the protocol over stdio, takes no arguments, and wants to
be started for the `mimas` language on `.mim` files. In Neovim, for example:

```lua
vim.lsp.config.mimas = {
    cmd = { "mimas-lsp" },
    filetypes = { "mimas" },
    root_markers = { "main.mim" },
}
vim.lsp.enable("mimas")
```
