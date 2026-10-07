# Changelog

Notable changes to mimas. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and mimas aims to follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html) once it reaches `1.0.0`. Until then, expect breaking changes in minor releases.

## [Unreleased]

### Added

- `Pact::*::member` reaches a member on every implementer of a pact and collects the results into an array. An associated function is called once per implementer, and a constant is read from each. See [Reaching every implementer](https://mim.as/reference/pacts.html#reaching-every-implementer).
- mimas now has a REPL which can be ran with `mimas repl` or just `mimas`. A demo of this is available on the [home page of the book](https://mim.as). Each input runs on top of the ones before it, and a trailing expression prints with its type.
- A pact constant can be read through a value that is only known by its pact (`thing.NAME` with `thing: Named`), which picks the value's own impl at runtime the way a method call does. It used to be a compile error.
- Constants can be enum variants, tuple structs and structs without fields (`const DOT = Shape::Dot;`, `const CIRCLE = Shape::Circle(4);`), as long as what they hold is constant. Constant arrays and tuples can now hold enum variants, structs and dicts, and a constant can be set from another one through a path (`const MINE = shapes::DOT;`). Each read gives its own value, so changing it never changes the constant. See [Enums, structs and collections](https://mim.as/reference/variables.html#enums-structs-and-collections).
- `Vm::set_fuel` limits how many ops a Vm can run before it faults with "ran out of fuel".
- `library::sandboxed` installs the standard library without the `fs`, `process` and `sys` modules, for hosts that run scripts they didn't write.
- `std::math` has `IVec2` and `IVec3`, vectors with `int` components, and `Vec2.to_ivec2` and `IVec2.to_vec2` to go between the two kinds. `vec2`, `vec3`, `ivec2` and `ivec3` are functions that create each one, so `ivec2(1, 2)` is `IVec2::new(1, 2)`.
- `int::random`, `float::random`, `bool::random`, `array.shuffle` and `array.choose` draw from a `library::Random` fixture, which a host can seed with `vm.fixture::<Random>().seed(n)` so a script gets the same numbers on every run.
- `name @ pattern` binds the whole matched value alongside the pattern's own bindings. Over a variant pattern the binding has the variant's type (`s @ Shape::Circle(_)` makes `s.0` reachable), which is how a match arm changes a variant's fields in place. See [Patterns](https://mim.as/reference/control-flow/match.html#patterns).
- `mimas-lsp` is a library as well as a binary. Its `Analysis` answers hover, definition, references, rename and inlay hints over a set of files without the protocol, and the `server` feature (on by default) adds the stdio server. The lunacade editor uses it for hover docs, go to definition, rename and type hints.
- mimas now has lunacade, a fantasy console example that you can play and edit at [mim.as/lunacade](https://mim.as/lunacade/). Its code is in `examples/lunacade`. See [lunacade](https://mim.as/introduction/lunacade.html).
- The `fancy` feature of `mimas`, on by default, renders errors with source snippets and colors through miette. If you'd rather have fewer dependencies, turning it off (`default-features = false`) drops 38 crates for faster builds and smaller binaries, and errors print as plain miette diagnostics instead.
- Pact constants can provide compile-time defaults (`const LEVELS: int = 8;`). Impls inherit a default unless they override it, with `Self` specialized for each implementer. See [Constants](https://mim.as/reference/pacts.html#constants).

### Changed

- Reading a constant's contents reuses a value built when the program loads instead of building it at every read. Binding, passing or storing it still gives its own value, and changing that value never changes the constant.
- Scripts run faster. mimas's benchmarks take 16% to 41% less time, with the biggest gains on function calls, struct field access, array reads and float math.
- Embedding mimas pulls in fewer crates: 98 instead of 101 with the default features, and 60 with only `export-api`.
- `print` and `dbg` now run through an `Output` fixture so hosts can redirect them.
- **Breaking**: `mimas docs` takes the manifest as `--manifest <path>`, which also works with `--mdbook`, instead of a second positional argument.
- **Breaking**: `ModuleApi::add_described` takes the function's documentation, so a native described at runtime shows up in `mimas docs` like one with a doc comment.

### Fixed

- Constants can use forward references in arithmetic, including in local blocks and across modules, even through long dependency chains.
- Pact names now follow module scope and visibility. A private pact in another module could be found by its bare name without an import.
- Using a pact name as a value, including inside parentheses or as an argument, is now a compile error instead of crashing the compiler.
- Struct patterns named through a module path, such as `shapes::Point { x }`, now resolve the struct and count toward match exhaustiveness. The path also checks the struct's visibility.
- Functions, closures and pact defaults with a non-unit return type now reject paths that finish without returning a value. Returns inside a `while` or `for` loop that may not run, or an argument skipped by an optional call, no longer count as returning on every path.
- Arrays, dicts and tuples can no longer be passed under a wider element type, such as passing `[int]` as `[int?]`. They share their contents, so the old check allowed writes through one binding to break another's type. Fresh collection literals still accept optional and pact element annotations.
- Function parameters are checked in the correct direction: a function taking `int` cannot be used as `(int?) -> int`, while one taking `int?` can be used as `(int) -> int`.
- Native method overload selection now checks that repeated type parameters agree, so a receiver with mixed element types can select the correct overload.
- An `if` or `match` with a nonreturning arm keeps the other arm's type. Loop breaks and collected values keep their optional type when any value can be `null`, regardless of their order.
- Nested optional types consistently flatten to one `?`, including when their inner type is resolved through inference.
- Integer arithmetic in constants now reports overflow and invalid shift counts at compile time. It used to wrap overflowing values or accept shifts of 64 bits or more.
- Ordering comparisons in constants compare integers exactly. Large integers could compare as equal after being rounded to floats.
- `&&` and `||` in constants now short-circuit, so `false && (1 ~/ 0 == 1)` and `true || (1 ~/ 0 == 1)` do not evaluate the division.
- Float constants now match runtime unary `+` and division by zero, and support `~/` and `%`.
- Some std methods panicked the host on bad arguments rather than raising a runtime error: `array.insert` past the end, `int.clamp` and `float.clamp` with `low` above `high`, `int::random` and `float::random` with an empty range, `array.sum` and `int.abs` overflowing, and `array::new_filled` with a length too large for an array.
- `array::new_filled` with a negative length is now a runtime error. It used to give an empty array.
- A value typed as a pact was accepted where a specific implementer was expected, so one struct could be read as another. It's now a type mismatch.
- The branches of an `if` or `match`, and the values a loop breaks with or collects, can be different types that share a pact. They used to be a type mismatch, even under a pact annotation.
- `Self` in a type mismatch reads as the type it stands for. Two different types could show as "expected Self but found Self".
- Calling a pact method with `?.` on an optional pact value (`thing?.name()` with `thing: Named?`) iced the compiler.
- `==` between two arrays or two tuples gave an array holding each element's comparison instead of a `bool`, and could panic the host when that result was used. It now compares them structurally, as `!=` already did.
- `!=` between an `int` and a `float` could be `true` when the two were equal (`i != f` with `let i = 1;` and `let f = 1.0;`).
- A struct named through a module path and used as a value (`let u = lib::Unit;`, or `let make = lib::Pair;` for a tuple struct) iced the compiler.
- A top-level `const` in a module couldn't use a name its file imported with `use` (`use lib::R; pub const X = R + 1;` reported an undefined variable).
- A top-level constant holding a struct could fail with a type mismatch when a field was an option or a result. With `x: int?`, `const A = P { x = null };` followed by `const B = P { x = 1 };` was rejected, because constants were checked before the struct's field types were known.
- A struct or dict as a parameter default (`fn spawn(at: Point = Point { x = 0 })`) iced the compiler. It's now a compile error. A default has to be a number, string, bool, `null`, or an array or tuple of those.
- A constant set to a comparison of two arrays or tuples compared how they were written, not their values, so `const SAME = [X] == [1];` was `false` with `const X = 1;`. It's now `true`. Setting a constant to a comparison of structs or dicts had the same problem and is now a compile error.
- A braced `use` that mixed a type or pact with a function or constant of the same module (`use module::{Pact, foo};`) could leave the type out of scope for the file's signatures and impls. A signature naming it reported an undefined variable, and an `impl Pact for ..` was dropped without an error, so the type didn't count as implementing the pact.
- Imported types or pacts, and pacts declared later in a module, could fail to resolve in pact signatures. Pact signatures now resolve imported names and forward pact references.
- Nested struct, enum, pact, and impl declarations now produce compile errors instead of being accepted or silently dropped.
- An impl's pact constant could have the wrong type (`const NAME = 4;` for a pact's `const NAME: str;`). Impl constants are now checked against the pact declaration, and an unannotated initializer takes its expected type from the pact, including collection literals and `null`.
- Fresh array, dict and tuple literals use the expected element types in function returns, branches, blocks and loop values, just as they do under a `let` annotation. For example, a function returning `[Pact]` can return `[Adt::new()]` or collect pact implementers in a loop. Existing collections keep their original element types.
- A closure without annotations takes its parameter and return types from its destination, as in `let f: (int) -> int = |n| n + 1;`, where before only a call argument's parameter type was used.

## [0.3.0] - 2026-09-26

This update focuses on usability for mimas, introducing a language server, [a full reference](https://mim.as/std.html) to the standard library, and the ability for users to generate their own documentation for their mimas projects, Rust types included. As always, please feel free to submit an issue if you have any issues or requests.

### Added

- mimas now has a language server, `mimas-lsp`. It offers diagnostics, hover (with `///` doc comments), go to definition, find references, rename, outlines, and inlay hints. The server correctly supports both your mimas types and your exported Rust types (including Bevy types if you're using its plugin). It comes bundled with the VS Code extension, which is available on the [marketplace](https://marketplace.visualstudio.com/items?itemName=mimas.mimas). See [Language Server](https://mim.as/introduction/lsp.html). ([#37](https://github.com/imlazyeye/mimas/pull/37), [#39](https://github.com/imlazyeye/mimas/pull/39), [#41](https://github.com/imlazyeye/mimas/pull/41), [#42](https://github.com/imlazyeye/mimas/pull/42), [#45](https://github.com/imlazyeye/mimas/pull/45))
- The book has a [Library Reference](https://mim.as/std.html) for the standard library. ([#49](https://github.com/imlazyeye/mimas/pull/49), [#57](https://github.com/imlazyeye/mimas/pull/57))
- `mimas docs <folder> [manifest]` writes a markdown page for every module and type in a host's API manifest. This is largely [written in mimas](./crates/cli/scripts/docs.mim)! ([#57](https://github.com/imlazyeye/mimas/pull/57))
- The `export-api` feature of `mimas`, on by default, writes the host's API to `target/mimas/<binary>.json` when the host runs from its cargo target dir. `mimas::write_api` writes it anywhere else. Turn the automatic write off with `default-features = false`. ([#41](https://github.com/imlazyeye/mimas/pull/41), [#45](https://github.com/imlazyeye/mimas/pull/45))
- `std::sys::file()`: Returns the absolute path to the file this function is written within, similar to Rust's `file!()` and Python's `__file__`. ([#32](https://github.com/imlazyeye/mimas/pull/32))
- `std::process::run_attached(cmd, args)`: Runs a command without capturing its output and returns its exit code. Unlike `std::process::run`, a non-zero exit isn't raised.
- Arrays gained `insert`, `deduped`, `reversed` and `to_dict`. ([#49](https://github.com/imlazyeye/mimas/pull/49))
- Dictionaries gained `get` and `get_or_insert`. ([#49](https://github.com/imlazyeye/mimas/pull/49))

### Changed

- **Breaking**: The project structure no longer utilizes a `main.mim` file. ([#40](https://github.com/imlazyeye/mimas/pull/40), [#45](https://github.com/imlazyeye/mimas/pull/45), [#56](https://github.com/imlazyeye/mimas/pull/56))
  - Scripts (files that don't declare a module) are individual executables and cannot see one another
  - Scripts can see and use any module in their project: the highest folder holding a `.mim` file in the cargo package or repository around them, including its sub-directories. The CLI and the language server agree on this. See [Projects](https://mim.as/reference/scripts.html#projects).
  - `mimas run <directory>` can still be used as long as your project has a single script
  - Projects with multiple scripts can still use `mimas run <script_file>`
- **Breaking**: The array method `flatten` is now `flat`. ([#49](https://github.com/imlazyeye/mimas/pull/49))
- **Breaking**: `Api::add_assoc_described` and `ModuleApi::add_described` take each parameter as a `(name, type)` pair. ([#49](https://github.com/imlazyeye/mimas/pull/49))
- The parser has been rewritten to be resilient, so each file can receive all of its syntax errors at once instead of one at a time. Embedding through the `mimas` crate still returns only the first. mimas also now uses `cargo fuzz` to guarantee the compiler's stability, and the crashes it found on malformed source are fixed. ([#33](https://github.com/imlazyeye/mimas/pull/33), [#34](https://github.com/imlazyeye/mimas/pull/34))
- Using a module, a library namespace, or a method without calling it as a value (i.e.: `let a = std::fs;`, `1.max;`) is now a type error with a hint, rather than an internal compiler error. ([#34](https://github.com/imlazyeye/mimas/pull/34))
- `mimas check` and `mimas build` check the scripts of a cargo package against the API manifest its host writes, the same way the language server does. See [Cargo Projects](https://mim.as/reference/scripts.html#cargo-projects). ([#56](https://github.com/imlazyeye/mimas/pull/56))

### Fixed

- A `return` or `break` without a value or a semicolon failed to parse at the end of a block (i.e.: `if x > 0 { return }`), and a `break` like that misread the statement after it. ([#34](https://github.com/imlazyeye/mimas/pull/34))
- A struct literal inside the parentheses, call, or index of an `if`, `while` or `for` header (i.e.: `if foo(Bar { a = 1 }) {}`) was rejected. ([#34](https://github.com/imlazyeye/mimas/pull/34))
- Calling through an unknown path (i.e.: `missing::f()`) reported "not a struct" with an internal type, rather than an undefined name. ([#38](https://github.com/imlazyeye/mimas/pull/38))
- Tuple indexing and field access on an enum (i.e.: `self.0` inside an enum's `impl`) passed type checking and then crashed at runtime. ([#44](https://github.com/imlazyeye/mimas/pull/44))
- A closure without parameters couldn't take a return type (`|| -> int { 1 }` failed to parse). ([#51](https://github.com/imlazyeye/mimas/pull/51))
- A closure's return type annotation wasn't checked against its body, and a `return` inside a closure was checked against the enclosing function's return type. ([#52](https://github.com/imlazyeye/mimas/pull/52))
- `break`, `continue` or `collect` inside a closure passed type checking when the closure sat in a loop, and then crashed the compiler. ([#53](https://github.com/imlazyeye/mimas/pull/53))

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
[0.3.0]: https://github.com/imlazyeye/mimas/releases/tag/v0.3.0
[0.2.0]: https://github.com/imlazyeye/mimas/releases/tag/v0.2.0
[0.1.0]: https://github.com/imlazyeye/mimas/releases/tag/v0.1.0
