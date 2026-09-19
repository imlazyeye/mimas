# Changelog

Notable changes to mimas. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and mimas aims to follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html) once it reaches `1.0.0`. Until then, expect breaking changes in minor releases.

## [Unreleased]

### Added

- `std::sys::file()`: Returns the absolute path to the file this function is written within, similar to Rust's `file!()` and Python's `__file__`.

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
