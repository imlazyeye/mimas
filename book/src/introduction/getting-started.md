# Getting started

mimas can be used directly at the CLI or from within your rust project.

## CLI Utility

To use mimas independently you can install its CLI utility.

```sh
cargo install mimas-cli
```

Mirroring cargo, you can use a `check`, `build`, and `run` command.

```admonish note
mimas doesn't "build" to any compiled files, it just runs off of scripts. The build command mostly exists for internal use to invoke the compiler!
```

```sh
mimas check my_script.mim # runs a script through the parsers/type checker
mimas check my_project    # you can also check a directory recursively for all .mim files

mimas run my_script.mim   # executes a script
mimas run my_project      # searches for the `main.mim` file and executes it

mimas my_script.mim       # you can also drop the "run" and just type "mimas"
```

## Rust Projects

To embed into your Rust project, add `mimas` as a dependency to your Cargo.toml.

```toml
[dependencies]
mimas = "0.1.0"
```

Compiling and execution is simple. A full guide can be found [here](../extension-with-rust.md)!

```rs
const SOURCE: &str = include_str!("my_script.mim");

let mut vm = mimas::compile_source(SOURCE).unwrap();
let _ = vm.run();
```