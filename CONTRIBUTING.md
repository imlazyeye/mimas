# Contributing

mimas is a young side project with one maintainer, so consider this a friendly note rather than a
real process. If something here looks fun to work on — a bug, a rough error message, a missing
stdlib method — go for it and open a PR. Reviews may be slow, but they'll come.

## The lay of the land

Source lives in [`crates/`](crates), one crate per stage of the pipeline:

```
.mim -> parse -> solve -> compile -> vm
```

- **`shared`** — common vocabulary (types, ids, spans) every crate depends on.
- **`parse`** — lexer + parser; source text to an AST.
- **`solve`** — the type checker; name resolution and inference.
- **`compile`** — lowers the checked program to bytecode.
- **`vm`** — the register-based VM that runs the bytecode.
- **`api`** / **`macros`** — the `#[mimas]` embedding surface for sharing Rust types.
- **`library`** — mimas's standard library.
- **`cli`** — the `mimas` binary.
- **`mimas`** — umbrella crate that re-exports the rest for host programs.

The [reference docs](https://mim.as) cover the language itself if you want to get a feel for it first.

## Working on it

```sh
cargo test                       # run the suite
cargo install --path crates/cli  # build the `mimas` binary locally
```

That's it — branch, change something, PR it. Thanks for taking a look.
