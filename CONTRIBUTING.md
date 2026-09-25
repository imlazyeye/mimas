# Contributing

mimas is a young side project with one maintainer, but contributions are very welcome. Please feel free to reach out for any questions if there is something you'd like to work on and/or if you need help figuring out how.

In the future we'll have a more robust documentation of each crate, but in the meantime, this serves as a basic guide.

**Please be sure to review our [Code of Conduct](./CODE_OF_CONDUCT.md) and our [LLM Usage Policy](./LLM_POLICY.md) before opening any issues or pull requests.**

## Basic structure

Source lives in [`crates/`](crates). Each stage in the pipeline has its own crate.

- **`shared`** -- common vocabulary (types, ids, spans) every crate depends on.
- **`parse`** -- lexer + parser; source text to an AST.
- **`solve`** -- the type checker; name resolution and inference.
- **`compile`** -- lowers the checked program to bytecode.
- **`vm`** -- the register-based VM that runs the bytecode.
- **`api`** / **`macros`** -- the `#[mimas]` embedding surface for sharing Rust types.
- **`library`** -- mimas's standard library.
- **`cli`** -- the `mimas` binary.
- **`mimas`** -- umbrella crate that re-exports the rest for host programs.

The [reference docs](https://mim.as) cover the language itself, though we don't yet have proper docs to cover the entire API.

## Running mimas

```sh
cargo test                       # run the suite (though cargo nextest is recommended)
cargo bench                      # runs our benchmarks
cargo install --path crates/cli  # install the `mimas` binary locally
```

## Tools

We also have some tools that are useful for development. They are written in `mimas`, so install it first!

### Fodder

Generates a randomized corpus of mimas code. Useful for benchmarking and stress testing the compiler.

```sh
mimas tools/fodder -- [line-count-target] [out-dir]
```

### Highlighter

Replaces blocks of mimas code in the book with rendered, colorized SVGs. This is ran as a preprocessor for `mdbook`, so you'd rarely call it manually.

### Token test generator

All of the tokens get tests automatically generated for them, so if you ever add/remove/change one, be sure to run this after.

```sh
mimas tools/autogen/main.mim
```

### Language benchmarking

Our suite for creating the [benchmarks in the book](./book/src/introduction/benchmarks.md) uses two shell scripts and a mimas script.

```sh
./benchmarks/compile-bench.sh                    # benchmarks compile times
./benchmarks/compare.sh [languages_to_run]       # benchmarks runtimes. they must be installed locally
mimas tools/inscribe_benchmarks/main.mim         # updates the graphs in the book
```

That said, the benchmarks are always ran on the same machine, so you shouldn't need to do this yourself beyond your own curiosity.

### Cargo mutants

You can run `cargo mutants` via `tools/test_mutants.sh`. Make sure you have it installed first!
