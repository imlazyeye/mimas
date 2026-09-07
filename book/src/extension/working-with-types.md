# Working With Types

Our goal is to allow you to work naturally while in Rust without having to think too much about how mimas itself works under the hood. Types that you use on your mimas-tagged Rust code can be marshalled into their mimas counterparts.

```admonish info title="The 'gc lifetime"
You'll often see the `'gc` lifetime showing up in native type signatures -- this is the lifetime tied to our garbage collector. Anything that lives within the Vm is tied to this lifetime.

You don't need to hand-write the lifetime on your function headers -- the macro will insert it if it is missing.
```

## The conversion table

| Rust type | mimas type | Notes |
| :-- | :-- | :-- |
| `bool` | `bool` | |
| `i8` through `i64`, `isize`, `u8` through `u64`, `usize` | `int` | Range-checked at the boundary (see [below](./working-with-types.md#integers-are-range-checked-at-the-boundary)) |
| `f32`, `f64` | `float` | `f32` narrows silently (see [below](./working-with-types.md#integers-are-range-checked-at-the-boundary)) |
| `String`, `&str`, `Str` | `str` | |
| `()` | `()` | `()` is just `null` at runtime |
| `Option<T>` | `T?` | `None` and `null` convert to one another. Trailing `Option` parameters may be omitted at the call site |
| `Raisable<T>` | `T!` | See [Runtime Errors](./runtime-errors.md) |
| `Vec<T>` | `[T]` | Copied in and out (see [below](working-with-types.md#collections-copies-and-borrows)) |
| `HashMap<String, T>` | `~{T}` | Copied in and out |
| Tuples, up to 8 elements | `(A, B, ...)` | Runtime shape is an array |
| A `#[mimas]` struct or enum | That type | |
| `anon::T` (and `U`, `V`, `W`) | A type linked per call site | See [below](./working-with-types.md#anonymous-types) |
| `Array`, `Dict` | The corresponding value | Zero-copy handles to the live value. |
| `Val` | Any value | Removes type analysis on anything it touches. Use [with caution](./working-with-types.md#admonition-tread-carefully-with-val). |

## Integers are range-checked at the boundary

A mimas `int` is an `i64`. Declaring any other integer type makes the conversion itself the validation: an argument that does not fit will fault at the call site with a normal runtime error.

```rust
#[native]
fn nth(items: &[anon::T<'gc>], index: usize) -> anon::T<'gc> { /* ... */ }
```

```text
x expected usize, received Int(-2)
```

This means a `usize` parameter is a free "must be a non-negative integer" check. When you want a friendlier (or recoverable) error instead, take `i64` and validate it yourself -- see [Runtime Errors](./runtime-errors.md).

```admonish warning title="f32 silently rounds"
The one asymmetry is `f32`: a mimas `float` is an `f64`, and converting to `f32` rounds silently rather than erroring. Declare `f32` only when that loss is acceptable.
```

## Collections

There are a few different ways to pass arrays and dictionaries into a function. 

| Parameter shape | Access |
| :-- | :-- |
| `&[Val]`, `&Vec<Val>`, `&mut Vec<Val>` | The live array, untyped elements |
| `&[anon::T<'gc>]`, `&Vec<anon::T<'gc>>`, `&mut Vec<anon::T<'gc>>` | The live array, elements typed to the receiver |
| `&DictMap<'gc>`, `&mut DictMap<'gc>` | The live dict, untyped values |
| `&DictMap<'gc, anon::T<'gc>>`, `&mut DictMap<'gc, anon::T<'gc>>` | The live dict, values typed to the receiver |
| `&[i64]`, `&Vec<String>`, etc. | A fresh, read-only `Vec<T>` |
| `&mut Vec<i64>`, etc. | compile error -- see below | 

```admonish warning title="Allocation costs"
Taking an owned `Vec` or `HashMap` will create a clone of your runtime data. However, **simply taking a reference is not enough to guarantee zero-cost references**. If your type signature will require marshalling the inner values of the collection then a clone is required to create the correct shape you ask for. This is why `&mut Vec<i64>` is made into a compile error -- the `Vec` you'd be mutating is one that was just minted for you and will be dropped after the call.

Marking your inner values as raw `Val` or an `Anon` will always allow for actual references.
```

## Anonymous types

mimas has no generics, but native signatures still need to express things like "the value pushed must match the array's element type." The `Anon` type allows you to write generic code while keeping your type safety.

The structure of this type is just `Anon<'gc, const N: u32>`, but rather than writing it out it is more common to use the shorthands available in `vm::anon`: `T`, `U`, `V`, and `W` -- aliases for slots 0 through 3. The explicit form works in any parameter position too (including the borrow shapes above), so a signature that somehow needs more than four independent slots can write `anon::Anon<'gc, 4>` and up. Within documentation we always write with a qualified usage (`anon::T<'gc>`) to make it clear that they are not true generics.

```admonish info title="Anons are local to their call"
An `anon::T` you use in one function has zero relation to an `anon::T` in another. Each time the solver encounters them it creates a freshly minted type ID that will only be mapped to the current call.
```

```rust
#[native]
fn push<'gc>(arr: &mut Vec<anon::T<'gc>>, val: anon::T<'gc>) { /* ... */ }
```

Calling `[1, 2].push(x)` constrains `x` to `int`; calling it on `[str]` constrains `x` to `str`. `U`, `V`, and `W` are additional independent slots.

`Val` is the opposite end of the spectrum: it accepts any value with no constraint, and the checker learns nothing from it. In practice you should just avoid it -- `anon::T` accepts everything `Val` does while keeping the checker engaged.

````admonish danger title="Tread carefully with Val!"
mimas uses all of the information the solver learns to structure its bytecode. Since `Val` is effectively a cheatcode for an `Any` type, you can easily trick mimas into running code it expects to be impossible. This will lead either to silent failures or panics, but this is not guaranteed -- in the future it may lead to UB.

As an example, this successfully compiles and executes:

```rust
// rust
#[mimas]
fn uh_oh(a: &mut Vec<Val<'gc>>, b: Val<'gc>) {
    a.push(b);
}
```
```mimas
// mimas
let a = [0, 1, 2];
uh_oh(a, "not good...");
print(a); // > [0, 1, 2, "not good..."]
```

Congrats, you've made mimas a dynamic language. Don't try this at home!
````

