# Variables & Constants

Local variables are introduced with `let`. A `let` binding can be reassigned.

```mimas
let a: int = 0;
let b = 1;  // type inferred as int

a = 10;     // reassignment is fine
```

```admonish tip title="Inference does the heavy lifting"
Type inference in mimas is strong. The only places you *must* write a type are function signatures and type declarations (`struct`, `enum`, `pact`). Everywhere else the compiler works it out from the value.

We still annotate freely throughout this book -- `let a: int = 0` -- purely to make each example's types obvious. You won't need most of them in real code.
```

## Reassignment

A `let` binding is reassignable, but its **type is fixed** at declaration. Assigning a value of a different type is an error -- use shadowing (below) if you genuinely want a new type.

```mimas
let count = 0;
count = 5;        // valid
count += 1;       // valid, count is now 6
count = "six";    // compile error: expected int, found str
```

## Shadowing

A new `let` may reuse a name already in scope. The new binding *shadows* the old one and can even change the type. This is a fresh variable, not a mutation of the original.

```mimas
let a: int = 0;
let a: str = "hello!"; // valid -- `a` is now a str
```

## Let else

`let`/`else` binds a [pattern](./control-flow/match.md) and runs an `else` block when the value doesn't match. The `else` has to diverge -- `return`, `break`, `panic`, and so on -- so execution only continues past it when the binding succeeded.

```mimas
let Shape::Circle(r) = shape else {
    return;
};
// `r` is in scope from here on
```

It accepts any pattern `match` does, including the `?` null-bind for [options](./options.md):

```mimas
let port? = lookup_port() else {
    panic("no port configured");
};
```

## Unused bindings

Prefixing a name with `_` marks it as intentionally unused.

```mimas
let _scratch = compute();
```

```admonish note title="No warnings yet"
`0.1.0` only reports errors, not warnings, so the `_` prefix is a no-op for now -- it documents intent for when unused-variable warnings land.
```

## Constants

Constants are declared with `const` and cannot be reassigned.

```mimas
const FOO: int = 0;
FOO = 1; // compile error: constants cannot be mutated after declaration
```

A constant's value must be computable at **compile time**. Literals, operators, and references to other constants are all fair game; anything that requires running code at runtime -- like a function call -- is not.

```mimas
const BAR = 0;
const FIZZ = BAR + 1;          // valid -- built from another const
const GREETING = "hello";      // valid -- a literal

const NOPE = some_call();      // compile error: constants must be known at compile time
```

```admonish todo
Our constant folding could likely handle evaluating whether a function is fully knowable at compile time, but that will come after `0.1.0`.
```
