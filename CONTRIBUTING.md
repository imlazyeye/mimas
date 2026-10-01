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

### Doctest

Generates tests for every \`\`\`mimas block in the crates and the book. These go into `crates/cli/tests/doctests/main.rs`, which should be left as a stub on the remote.

- Lines in codeblocks that start with a `#` will be hidden when using the LSP or reading the book. This allows you to set up your types ahead of your example code so that you can just show the important part.
- You can write code blocks that are intended to fail by adding `compile error:` or `runtime error:` followed by the exact error message expected. If the message does not match, or the code does not error, the test will fail.
- You can write `no_run` after your \`\`\`mimas opening to instruct the test to build, but not execute. This should be done for examples that run code that is not safe to run randomly (for example, various `std::fs` commands).
- You can write `ignore` after your \`\`\`mimas opening to fully ignore a code block. This is generlly frowned upon, but makes sense in particular scenarios when it would be impractical to set up the types needed beforehand, or when showing illegal code without labeling its particular error message.

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
