# Getting started

mimas can be used directly at the CLI or from within your rust project.

## CLI Utility

To use mimas independently you can install its CLI utility.

```sh
cargo install mimas-cli

mimas check my_script.mim   # parse + type-check
mimas run my_script.mim     # execute (or just `mimas my_script.mim`)
mimas run my_project        # runs the project's one script, with its modules
mimas my_project            # running with no subcommand defaults to `run`
mimas repl                  # runs the repl
mimas                       # also runs the repl
```

## Rust Projects

To embed into your Rust project, add `mimas` as a dependency to your Cargo.toml.

```toml
[dependencies]
mimas = "0.3.0"
```

Compiling and execution is simple. A full guide can be found [here](../extension-with-rust.md)!

```rs
const SOURCE: &str = include_str!("my_script.mim");

let mut vm = mimas::compile_source(SOURCE).unwrap();
let _ = vm.run();
```