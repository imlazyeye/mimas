<div align="center">

<img src="book/src/brand/icon-color.svg" alt="mimas" width="140" />

# mimas

**A flexible, statically typed scripting language for Rust.**

[![docs](https://img.shields.io/badge/docs-mim.as-3d8ef7?style=flat-square)](https://mim.as)
[![version](https://img.shields.io/badge/version-0.1.0-66E8FF?style=flat-square)](https://crates.io/crates/mimas)
[![built with Rust](https://img.shields.io/badge/built_with-Rust-dea584?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![license](https://img.shields.io/badge/license-MIT_OR_Apache--2.0-8E9BFF?style=flat-square)](#license)
[![ci](https://img.shields.io/github/actions/workflow/status/imlazyeye/mimas/test.yml?branch=main&style=flat-square&label=ci)](https://github.com/imlazyeye/mimas/actions/workflows/test.yml)
[![tests](https://img.shields.io/badge/tests-1%2C700%2B_passing-5BD6B0?style=flat-square)](https://mim.as)

[**Get started**](https://mim.as/introduction/getting-started.html) ·
[Tour the language](https://mim.as/introduction/tour.html) ·
[Why mimas?](https://mim.as/introduction/why-mimas.html) ·
[Benchmarks](https://mim.as/introduction/benchmarks.html) ·
[Reference](https://mim.as/reference.html)

</div>

---

mimas is a statically typed, embeddable scripting language for Rust. It carries over much of Rust's
syntax and ergonomics, reshaping the rest to deliver what a scripting layer is good for: fast
iteration, quick compile times, and runtime flexibility — without trading away the safety that keeps
you out of the debugger.

```mimas
enum Shape {
    Circle(float),
    Rect(float, float),
}

fn area(shape: Shape) -> float {
    match shape {                          // exhaustive — miss a variant and it won't compile
        Shape::Circle(r) => r * r * std::math::PI,
        Shape::Rect(w, h) => w * h,
    }
}

let shapes = [Shape::Circle(1.0), Shape::Rect(2.0, 3.0)];
let total = 0.0;
for s in shapes { total += area(s); }
print(f"area of {shapes.len()} shapes: {total}");
```

## What you get

| | |
| --- | --- |
| 🪶&nbsp;&nbsp;**Flexible** | Inference writes your types, compiles stay fast, and errors point toward the fix instead of just turning you away. Garbage collected — no borrow checker, no lifetimes. |
| 🛡️&nbsp;&nbsp;**Typed** | Static typing with inference, user-defined types, exhaustive [pattern matching](https://mim.as/reference/control-flow/match.html), and `T?` option safety so an unexpected `null` can't reach you. |
| 🧩&nbsp;&nbsp;**Extendable** | Share Rust types and functions with the `#[mimas]` macro — they're type-checked just like native ones. |
| ✅&nbsp;&nbsp;**Robust** | Every panic is treated as a bug, top to bottom. Over **1,700 tests** (the tests are tested, via [cargo mutants](https://mutants.rs)), with clear diagnostics powered by [miette](https://github.com/zkat/miette). |

Read the [full tour](https://mim.as/introduction/tour.html) for a quick pass over the whole language.

## Quick start

**At the command line** — mirroring cargo, with `check`, `build`, and `run`:

```sh
cargo install mimas-cli

mimas check my_script.mim   # parse + type-check
mimas run my_script.mim     # execute (or just `mimas my_script.mim`)
mimas run my_project        # runs the project's main.mim
```

**Embedded in a Rust project** — add `mimas` and compile a script in two lines:

```rust
const SOURCE: &str = include_str!("my_script.mim");

let mut vm = mimas::compile_source(SOURCE).unwrap();
let _ = vm.run();
```

Sharing your own Rust types is one attribute away:

```rust
#[mimas]
struct User(String);

#[mimas]
impl User {
    fn greet(self) { println!("Hello, {}!", self.0); }
}
```

```mimas
let user = User("mimas");
user.greet(); // -> Hello, mimas!
```

The full embedding guide lives at [Extension with Rust](https://mim.as/extension-with-rust.html).

## Performance

mimas compiles `.mim` source to bytecode for a register-based VM with a lifetime-safe
([gc-arena](https://github.com/kyren/gc-arena)) heap. It's ahead of Rune and Rhai on every test we
run, and within shooting range of Luau, a mature C++ runtime. See the
[full benchmarks](https://mim.as/introduction/benchmarks.html) for the methodology and numbers.

The compiler is quick too: the whole pipeline (parse, type-check, lower, emit bytecode) runs at
roughly **400,000 lines per second**, so for scripts there's effectively no compile step you'd notice.

## Examples

Runnable projects live in [`examples/`](examples):

- [`extension`](examples/extension) — sharing Rust structs, enums, methods, and fallible functions with a script via `#[mimas]`.
- [`game-loop`](examples/game-loop) — driving a script from a host game loop, using fixtures and `FreezeCell` to safely hand mimas a `&mut` to host state.

## Editor support

A VS Code extension lives in [`tools/vscode`](tools/vscode) — syntax highlighting, snippets, and
language configuration for `.mim` files. Build it with `vsce package` and install the `.vsix`. The TextMate grammar it uses
([`tools/highlighter`](tools/highlighter)) is written in mimas, and is the same one that colors the
docs.

## Status

mimas is `0.1.0` and a young side project. The design is committed and the language compiles,
type-checks, and runs end-to-end, but nothing beyond the design is promised to be stable yet. Expect
sharp edges, expect things to move — and if a little language taking on a big problem sounds fun,
[stubborn thoughts and contributions](https://mim.as/introduction/why-mimas.html) are very welcome.

## How it's built

A single pipeline turns source into a running program, split across a handful of crates in
[`crates/`](crates):

```
.mim -> parse -> solve (type check) -> compile (lower + bytecode) -> vm (register VM)
```

`shared` carries the common vocabulary, `api` and `macros` back the `#[mimas]` embedding surface,
`library` is the standard library, and `cli` is the `mimas` binary.

## License

Dual licensed under your choice of [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE).

Built on the shoulders of [gc-arena](https://github.com/kyren/gc-arena),
[miette](https://github.com/zkat/miette), and [chompy](https://github.com/imlazyeye/chompy); the
`gc-arena` singleton and freeze patterns are adapted from [fabricator](https://github.com/kyren/fabricator).
</content>
</invoke>
