# Blocks & Scope

Like Rust and Zig, a block in mimas is an **expression**. Curly braces group statements, and if the last thing inside -- before the closing brace -- is a bare expression (no trailing `;`), the block evaluates to it.

```mimas
let a = {};          // -> ()  (no trailing expression)
let b = { 0 };       // -> 0
let c = {
    let x = 1;
    let y = 2;
    x + y            // no semicolon: this is the block's value
};                   // -> 3
```

Because blocks are expressions, the same rule powers [`if`](./control-flow/if-else.md), [`match`](./control-flow/match.md), and [loops](./control-flow/loops.md) -- they can all produce values.

## Scope

mimas is lexically scoped: a block defines a scope. Code can read and reassign variables from any block that encloses it, but not the other way around. When a block ends, the variables it declared fall out of scope.

```mimas
let a = 0;
{
    let b = a; // valid -- `a` is visible from the enclosing scope
}
let c = b;     // compile error: `b` is not defined out here
```

A `let` inside a block is a brand-new binding. It does not disturb a same-named variable in an outer scope -- that's [shadowing](./variables.md#shadowing). Reassignment (`=`, without `let`), on the other hand, reaches outward to the existing variable.

```mimas
let a = 0;
let b = 0;
{
    a = 1;     // reassigns the outer `a`
    let b = 1; // new binding, shadows the outer `b` only inside this block
}
// -> a is 1, b is 0
```
