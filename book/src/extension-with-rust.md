# Extension with Rust

mimas is designed to live inside a Rust program, and the boundary between the two is meant to be easy to cross: Rust can provide its types, functions, and systems, mimas takes it from there. This chapter covers that boundary -- how to expose your Rust code to mimas, what happens to values as they cross, and how to handle the things that can go wrong.

The short version is that a single attribute is usually all it takes:

```rust
// rust
#[mimas]
fn roll(sides: i64) -> i64 {
    /* ... */
}
```

```mimas
// mimas
print(roll(20));
```

Nothing about this is dynamic: the signature you write in Rust becomes the signature the mimas compiler enforces at every call site, so a script that misuses your API fails to compile rather than failing at runtime.

| Page | Covers |
| :-- | :-- |
| [Basics](./extension/basics.md) | The `mimas` macro: functions, constants, types, and modules |
| [Working With Types](./extension/working-with-types.md) | How Rust types map to mimas types, and what each costs |
| [Runtime Errors](./extension/runtime-errors.md) | The ways a native function can fail, recoverably or not |
| [Fixtures & Context](./extension/fixtures-and-context.md) | How to communicate data from the host into the native functions you provide |
| [Freeze](./extension/freeze.md) | Taking Fixtures a step further and enabling mutable access to the host.
| [Advanced Usage](./extension/advanced-usage.md) | Registering by hand for the cases the macro can't infer |
