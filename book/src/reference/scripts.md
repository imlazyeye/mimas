# Scripts

There is no implicit `main` function in mimas. Unlike Rust, a script just runs top to bottom -- write whatever code you like at the file's root and execute it directly, the way you would with Python or JavaScript.

```mimas
// hello.mim
print("Hello, world!"); // runs the moment you execute the file
```

```sh
mimas run hello.mim
```

Top-level statements run in order, and top-level `fn`s, `struct`s, and similar declarations are available throughout the file regardless of where they're declared.

```mimas
print(greet("world"));      // works -- `greet` is visible across the whole file

fn greet(name: str) -> str {
    f"hello, {name}!"
}
```

```admonish note title="Scripts run; modules organize"
A file that opens with a [`module`](./modules.md) declaration is a library, not a script -- it can't carry top-level expressions to execute. This split keeps "the thing you run" distinct from "the code you structure for reuse."
```

## Projects

A project is a directory. Its scripts and modules can sit anywhere in it, subdirectories included, without any setup. `mimas run script.mim` will compile all the modules in that script's project and then execute its code. `mimas run ./some/dir` treats the directory as the project, and can be used if it has only one script file to choose from.

```sh
mimas run my_project           # runs the one script in my_project
mimas run my_project/game.mim  # or picks one of many
```

A script's project is the highest folder holding a `.mim` file in the cargo package or git repository around it, along with every folder below that. A script outside of both only sees the `.mim` files next to it. Another cargo package or a cargo `target` directory is never part of a project. The [language server](../introduction/lsp.md) follows the same rules.

### Cargo Projects

A project inside a cargo package is usually run by that package's Rust host, which can give its scripts functions, types, and constants of its own. By default, `mimas` has the `export-api` feature enabled. With this on, the first time your host compiles scripts, it writes a manifest of everything it installed (the standard library and your host's own items) to `target/mimas/<binary_name>.json`. Each binary and example writes its own, so hosts sharing a target dir don't overwrite each other. This only happens while the host runs from inside its cargo target dir, so shipped builds never write one, and neither do tests or benches. You can turn the automatic write off with `default-features = false`, and you're always free to export it yourself with `mimas::write_api` (such as a copy you ship for modders):

```rust
let library = mimas::Vm::new().install_library(mimas::library::std);
mimas::write_api(&library, "scripts/api.json".as_ref())?;
```

`mimas check`, `mimas build` and the [language server](../introduction/lsp.md#host-apis) find the manifest on their own, checking a project's scripts against the newest one written by a binary of its package. Until the host has run, they warn and fall back to the standard library alone. After you change your host's API, run it again to update the manifest. A manifest only works with the version of mimas that wrote it.
