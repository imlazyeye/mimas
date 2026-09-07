# If & If Let

An `if` expression tests a `bool` condition and runs its block when the condition holds. An optional `else` -- including chained `else if` -- handles the other case.

```mimas
if ready {
    launch();
}

if ready {
    launch();
} else if waiting {
    hold();
} else {
    abort();
}
```

## As an expression

Because `if` is an expression, it can produce a value. When you use it that way, an `else` is **required** -- without one, the `if` might produce nothing, so the only value it's allowed to have is `()`.

```mimas
let tier: int = if vip {
    0
} else {
    1
};

let c = if vip { 0 }; // compile error: an `if` used as a value must have an `else`
```

Each branch must agree on a type -- though a diverging branch (one that `return`s, `break`s, or `panic`s) contributes the [never type](../special-types.md) and bows out of that agreement:

```mimas
let port: int = if configured {
    configured_port()
} else {
    panic("no port configured"); // `!` -- doesn't fight the `int` from the other arm
};
```

## If let

`if let` swaps the `bool` test for an **option test**. It evaluates an expression and, if the result is not `null`, binds the unwrapped value and runs the block. An `else` runs when the value was `null`.

```mimas
if let port = lookup_port() {
    // runs only when `lookup_port()` was not null;
    // `port` is the non-null value in here
    connect(port);
} else {
    use_default();
}
```

This is the ergonomic way to "check and use" an optional in one step, instead of testing for `null` and then unwrapping separately. The same pattern-driven form exists for loops as [`while let`](./while.md#while-let).
