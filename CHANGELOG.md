# Changelog

Notable changes to mimas. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and mimas aims to follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html) once it reaches `1.0.0`. Until then, expect breaking changes in minor releases.

## [Unreleased]

### Added

- mimas now has a language server, `mimas-lsp`, installed with `cargo install mimas-lsp`. It offers diagnostics, hover (with `///` doc comments), go to definition, find references, rename, outlines, and inlay hints. It comes bundled with the VS Code extension, which is available on the [marketplace](https://marketplace.visualstudio.com/items?itemName=imlazyeye.mimas). A Rust host run from its cargo target dir writes its API to `target/mimas/<binary>.json`. The server finds the manifests of the cargo package a script sits in through `cargo metadata` and loads the newest, so scripts are checked against their own host's functions, types and constants. The VS Code extension can point it at another file with `mimas.apiPath` or the "mimas: Select Host API Manifest" command, and restart it with "mimas: Restart Language Server". See [Language Server](https://mim.as/introduction/lsp.html). ([#37](https://github.com/imlazyeye/mimas/pull/37))
- The `export-api` feature of `mimas`, on by default, writes the host's API to `target/mimas/<binary>.json` when the host runs from its cargo target dir. `mimas::write_api` writes it anywhere else. Turn the automatic write off with `default-features = false`.
- With the `bevy` feature, hover shows the `///` docs of reflected components, resources, and enum variants, Bevy's own types included.
- `std::sys::file()`: Returns the absolute path to the file this function is written within, similar to Rust's `file!()` and Python's `__file__`.
- `std::process::run_attached(cmd, args)`: Runs a command without capturing its output and returns its exit code. Unlike `std::process::run`, a non-zero exit isn't raised.
- The book has a [Library Reference](https://mim.as/std.html) for the standard library. It's generated from the doc comments in `crates/library` every time the book builds using a script written in mimas ([std.mim](./tools/book/scripts/std.mim)). This tool will be linked into the `mimas` cli in a future update so that users can use it on their own projects. It can export all items from both your Rust host and your scripts.
- Arrays gained `insert`, `deduped`, `reversed` and `to_dict`.
- Dictionaries gained `get` and `get_or_insert`.

### Changed

- **Breaking**: The project structure no longer utilizes a `main.mim` file.
  - Scripts (files that don't declare a module) are individual executables and cannot see one another
  - Scripts can see and use any module in their project: the highest folder holding a `.mim` file in the cargo package or repository around them, including its sub-directories. The CLI and the language server agree on this. See [Projects](https://mim.as/reference/scripts.html#projects).
  - `mimas run <directory>` can still be used as long as your project as a singular script
  - Projects with multiple scripts can still use `mimas run <script_file>`
- **Breaking**: The array method `flatten` is now `flat`.
- **Breaking**: `Api::add_described` and `Api::add_assoc_described` take each parameter as a `(name, type)` pair.
- The parser now recovers from syntax errors, so the `mimas` CLI reports every syntax error in a file instead of stopping at the first. Embedding through the `mimas` crate still returns only the first.
- Using a module, a library namespace, or a method without calling it as a value (i.e.: `let a = std::fs;`, `1.max;`) is now a type error with a hint.
- `mimas check` and `mimas build` check the scripts of a cargo package against the API manifest its host writes, the same way the language server does. See [Cargo Projects](https://mim.as/reference/scripts.html#cargo-projects).

## [0.2.0] - 2026-09-17

This marks the first update for mimas, which is primarily focused on the new [Bevy](https://bevy.org) plugin. Like everything else, it is very young, so it will no doubt require further work and expansion. Please submit an issue if you find any problems or have any requests!

### Added

- mimas now has direct support for `Bevy` with its own plugin housed in `crates/bevy`. It is an optional feature that can be enabled on `mimas` dependencies with `features = ["bevy"]`. Scripts are assets, so they hot reload, and they are type-checked against your app's components and resources when they load. For more information and a working demo, check out the [website](https://mim.as/extension/bevy.html). ([#30](https://github.com/imlazyeye/mimas/pull/30))
- Strings can now be indexed by an integer (i.e.: `"hello"[1] == "e"`). ([#1](https://github.com/imlazyeye/mimas/pull/1))
- Strings now support ordering comparisons, following the same implementation as Rust (i.e.: `"a" < "b"`). ([#2](https://github.com/imlazyeye/mimas/pull/2))
- `std::math` now provides glam's `Vec2`, `Vec3` and `Quat`. ([#14](https://github.com/imlazyeye/mimas/pull/14))
- Floats gained `clamp`, `signum`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `pow`, `exp` and `ln`, and `std::math` gained the `TAU` and `E` constants. ([#13](https://github.com/imlazyeye/mimas/pull/13))
- The host can now hold onto a function or closure a script hands it. `ctx.stash` turns one into a `Stashed`, which survives collection and keeps the value alive, and `vm.call_value` runs it once the host has control back -- enough to build a callback system. See [Function Values](https://mim.as/extension/function-values.html). ([#22](https://github.com/imlazyeye/mimas/pull/22))
- `Vm::root` hands the host the script's items -- its fns, consts and types, and the same for every module. `Vm::registry` turns a Rust type into its `Ty`. A fn found this way can be checked once with `Function::check` and then called through `Vm::call_function`. ([#17](https://github.com/imlazyeye/mimas/pull/17))
- `Api::add_adt_described`, `Api::add_assoc_described` and `Api::add_described` register types and natives whose shape is only known at runtime, such as ones described through reflection. ([#19](https://github.com/imlazyeye/mimas/pull/19))

### Changed

- **Breaking**: `Vm::call_fn` is replaced by `Vm::call`, which takes a fully qualified path (including `::`), arguments, and a return type, all checked against the signature before the call. `vm.call_fn("call_me")` becomes `vm.call::<()>("call_me", ())`. `Vm::call_then` is the same call plus a closure, which it runs with the arena still open, so anything a native put aside during the call can still be read before it is collected. ([#17](https://github.com/imlazyeye/mimas/pull/17))
- The minimum supported Rust version is now 1.95. ([#12](https://github.com/imlazyeye/mimas/pull/12))

### Fixed

- Calling an associated function with arguments via dot syntax could cause an ICE. ([#11](https://github.com/imlazyeye/mimas/pull/11))
- Pacts did not handle skolems, which allowed invalid type combinations to pass through. ([#25](https://github.com/imlazyeye/mimas/pull/25))
- A `_` in an or-pattern counted as a binding. ([#16](https://github.com/imlazyeye/mimas/pull/16))
- `!in` swallowed identifiers that start with `in` (i.e.: `!inside`). ([#15](https://github.com/imlazyeye/mimas/pull/15))
- A dynamic call with the wrong arity panicked the host rather than raising a runtime error. ([#21](https://github.com/imlazyeye/mimas/pull/21))

### New Contributors

- Hayden Flinner [@haydenflinner](https://github.com/haydenflinner)

## [0.1.0] - 2026-09-07

The first public release. Everything is new, so rather than list it all, here's the shape of what already exists.

### The language

Static typing with inference, `struct`/`enum` user-defined types, exhaustive pattern matching, `T?` options and `T!` results, closures, modules with privacy, and pacts (a scoped stand-in for traits). Garbage collected via [gc-arena](https://github.com/kyren/gc-arena) -- no borrow checker, no lifetimes.

### The tooling

- `mimas` CLI with `check`, `build`, and `run`.
- Embedding via the `mimas` crate: `compile_source`, `compile_files`, and the `#[mimas]` attribute
  for sharing Rust types, methods, and functions.
- A VS Code extension in [`tools/vscode`](tools/vscode) (not yet on the marketplace).

### Known limitations

These are rejected with a clear error rather than misbehaving, and are documented where they'd otherwise surprise you:

- Pact **constants** don't dispatch -- they can only be read off a concrete type, not a pact-typed
  value. Pact methods do dispatch normally.
- `Self::` doesn't resolve inside a pact default body. Lowercase `self` works, as does `Self::`
  inside an ordinary `impl`.
- Strings can't be indexed (`s[0]`); iterate with `for c in s`.
- A `const` array or tuple can't contain dictionaries.
- Mutating a collection while iterating it is not caught, and can loop forever.
- No generics, no async, and dictionary keys must be strings.

<!-- next-release -->
[0.2.0]: https://github.com/imlazyeye/mimas/releases/tag/v0.2.0
[0.1.0]: https://github.com/imlazyeye/mimas/releases/tag/v0.1.0
