# Advanced Usage

The `mimas` macro has two jobs: it translates your item into a format that can satisfy the type requirements of API installation, _and_ it automatically performs that installation for you. In this section we'll break down how to do the first part without the second so that you can handle installation manually.

The first reason you may want to approach things this way is if you are using multiple crates to build/use your API -- [see why here](./basics.md#admonition-magic-is-dangerous).

The second reason is to have more flexibility in _how_ your item is installed. Some registrations can't be inferred from the item alone: methods on the core types, function overloads, and custom names all require manual control.

## The `native` macro

`#[native]` is the conversion half of `#[mimas]`. It rewrites your function into the shape the VM can call -- everything from [Working With Types](./working-with-types.md) applies, including `Ctx`, the borrow shapes, and `anon`, but no registration happens; the function just sits there waiting for you to install it.

```rust
// rust
use mimas::native;

#[native]
fn shout(s: &str) -> String {
    format!("{}!", s.to_uppercase())
}
```

## Installer functions

Installation happens through an **installer**: a function handed an `Api` that registers everything your program offers. You pass it to the Vm at compile time, and the usual pattern is one installer that pulls in the standard library and then your own:

```rust
// rust
use mimas::Api;

fn installer(api: &mut Api) {
    mimas::library::std(api);
    my_library(api);
}

fn my_library(api: &mut Api) {
    api.add_method(shout);
    // ... the rest of your program's API
}

fn main() {
    let mut vm = mimas::Vm::compile(r#"print("hey".shout());"#, installer).unwrap();
    vm.run().unwrap(); // > HEY!
}
```

A few things worth noticing:

- The standard library only arrives if you ask for it -- that's the `mimas::library::std(api)` line, and you almost always want it first. (`mimas::compile_source` is nothing more than `Vm::compile` with `library::std` alone as the installer.)
- Going manual doesn't turn the magic off. Any `mimas`-tagged items that made it into your binary still install themselves, interleaved with your installer: tagged types first, then your installer, then tagged functions. That ordering means your installer can reference `#[mimas]` types, and `#[mimas]` functions can reference yours -- the two styles mix freely.

## The `Api` surface

Everything an installer can do:

| Call | Installs | mimas name |
| :-- | :-- | :-- |
| `api.add(f)` | A function in the prelude | The Rust ident |
| `api.add_named("name", f)` | A function in the prelude | `"name"` |
| `api.add_method(f)` | A method on its first parameter's type | The Rust ident |
| `api.add_method_named("name", f)` | A method, [overloadable](#overloading) | `"name"` |
| `api.add_assoc(ty, f)` | An associated function on `ty` | The Rust ident |
| `api.add_adt::<T>()` | A [derived](#structenum-conversion) struct or enum | The type's name |
| `api.constant("NAME", ty, value, doc)` | A constant in the prelude | `"NAME"` |
| `api.assoc_constant(ty, "NAME", ...)` | An associated constant on `ty` | `"NAME"` |
| `api.module("a::b").add(f)` | A function, type, or constant inside a module | As above |

`api.module` returns a scoped handle, so module placement is one extra call rather than a different API: `api.module("game").add(spawn)` is the manual spelling of `#[mimas(game)]`. Methods and associated functions take no module -- they belong to their receiver.

## Struct/Enum Conversion

`#[mimas]` on a type is just an alias for our derive macros, `MimasStruct` and `MimasEnum`, plus an automatic `add_adt`. Done by hand:

```rust
// rust
use mimas::{MimasStruct, native};

#[derive(MimasStruct)]
struct Player {
    name: String,
    health: i64,
}

#[native]
fn spawn(name: String) -> Player {
    Player { name, health: 100 }
}

fn installer(api: &mut Api) {
    mimas::library::std(api);
    api.add_adt::<Player>();
    api.add(spawn);
}
```

```admonish warning title="Order matters"
A type must be registered before anything that mentions it: `add_adt::<Player>()` has to run before `add(spawn)`, or installation panics. The macro's deferred registration handles this ordering for you (types always install first); by hand, the ordering is yours to keep.
```

## Methods on core types

`api.add_method` attaches a function to whatever type its first parameter is -- that parameter becomes the receiver. This is the only way to put methods on the core types, and it's how the entire standard library is built:

```rust
// rust
#[native]
fn abs(n: i64) -> i64 {
    n.abs()
}

api.add_method(abs);
```

```mimas
// mimas
print((-5).abs()); // > 5
```

The receiver follows the [conversion table](./working-with-types.md#the-conversion-table) like any other parameter, so the borrow shapes pick which type you're attaching to: a first parameter of `&[anon::T<'gc>]` makes an array method, `&DictMap<'gc>` a dict method, and a `#[derive(MimasStruct)]` type works too (though for your own types, [`#[mimas] impl`](./basics.md) is the comfortable path).

## Associated functions

Associated functions have no `self` parameter to introspect, so you name the receiver yourself with a `Ty`:

```rust
// rust
use mimas::Ty;

#[native]
fn roll(sides: i64) -> i64 { /* ... */ }

api.add_assoc(Ty::Int, roll);
```

```mimas
// mimas
print(int::roll(20));
```

For your own registered types, `api.ty_of::<Player>()` hands back the `Ty` to pass -- just make sure `add_adt` has already run. That said, you'll rarely need it: if your goal is associated functions on your own type, tagging an [`impl` block](./basics.md) gets you there without any of this.

## Overloading

Registering multiple functions under one method name with `add_method_named` creates an overload, dispatched by the shape of the receiver. This is, for example, how `max` works on both `[int]` and `[float]`:

```rust
// rust
#[native]
fn max_int(arr: &[i64]) -> Option<i64> {
    arr.iter().max().copied()
}

#[native]
fn max_float(arr: &[f64]) -> Option<f64> {
    arr.iter().copied().reduce(f64::max)
}

api.add_method_named("max", max_int);
api.add_method_named("max", max_float);
```

```mimas
// mimas
print([1, 5, 3].max()!);     // > 5
print([1.0, 2.5].max()!);    // > 2.5
```

At each call site the compiler unifies the actual receiver against every candidate's receiver shape and requires exactly one winner: no match is an ordinary type mismatch, more than one is an ambiguity error. That's why the typed-element shapes matter here -- `&[i64]` registers the precise shape `[int]`, which is what gives the compiler something to choose by. An `&[anon::T<'gc>]` candidate matches *every* array, so it can't share a name with anything else.
