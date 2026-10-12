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

## Conventional Commits

Our commit style is built largely from the spec for [Conventional Commits](https://www.conventionalcommits.org/). We apply them equally onto the titles of our pull requests. Don't worry about getting it perfect first try; we tend to squash PRs, so a maintainer will change adjust the name of the new commit if needed.

### Prefixes

- `feat`: A new user-facing feature.
- `fix`: A fix for any sort of bug or mistake.
- `refactor`: An adjustment to the codebase to improve legibility, performance, etc.
- `tools`: Modifications focused on the tools this repo utilizes.
- `docs`: Edits to the book or the documentation of the crates/standard library.
- `ci`: Work focused on our GitHub Actions.
- `test`: Changes and additions to the unit tests.
- `wip`: Commits that are in an unfinished state. It is okay to include these in your pull requests, but `wip` should never appear on `main`.
- `chore`: Anything and everything else if the above do not fit.

### Scope

The area of the project this commit is working within, typically the crate itself. These are encouraged but ultimately optional if there is no meaningful scope to add. Scopes go in parenthesis after your prefix but before your `:`.

```
feat(vm): functions can be called directly by hosts
docs(std): added more examples to std::math::Vec2
ci: move `miri` test to only occur on main
```

### Breaking Changes

Any commit that introduces a breaking change must include a `!` prior to the `:`.

```
refactor(std)!: moved the `PI` constant out of the `math` module and into `float`
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
