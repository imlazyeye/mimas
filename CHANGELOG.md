# Changelog

Notable changes to mimas. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and mimas aims to follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html) once it reaches
`1.0.0`. Until then, expect breaking changes in minor releases.

## [0.1.0] - 2026-09-07

The first public release. Everything is new, so rather than list it all, here's the shape of what
already exists.

### The language

Static typing with inference, `struct`/`enum` user-defined types, exhaustive pattern matching,
`T?` options and `T!` results, closures, modules with privacy, and pacts (a scoped stand-in for
traits). Garbage collected via [gc-arena](https://github.com/kyren/gc-arena) —- no borrow checker,
no lifetimes.

### The tooling

- `mimas` CLI with `check`, `build`, and `run`.
- Embedding via the `mimas` crate: `compile_source`, `compile_files`, and the `#[mimas]` attribute
  for sharing Rust types, methods, and functions.
- A VS Code extension in [`tools/vscode`](tools/vscode) (not yet on the marketplace).

### Known limitations

These are rejected with a clear error rather than misbehaving, and are documented where they'd
otherwise surprise you:

- Pact **constants** don't dispatch — they can only be read off a concrete type, not a pact-typed
  value. Pact methods do dispatch normally.
- `Self::` doesn't resolve inside a pact default body. Lowercase `self` works, as does `Self::`
  inside an ordinary `impl`.
- Strings can't be indexed (`s[0]`); iterate with `for c in s`.
- A `const` array or tuple can't contain dictionaries.
- Mutating a collection while iterating it is not caught, and can loop forever.
- No generics, no async, and dictionary keys must be strings.

[0.1.0]: https://github.com/imlazyeye/mimas/releases/tag/v0.1.0
