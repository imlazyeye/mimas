# The Unit & Never Types

Three things in mimas stand in for "no ordinary value here," and they mean genuinely different things:

- **`null`** -- there *could* be a value, but right now there isn't. It's a real value you can hold and test, and it only appears behind an [option](./options.md) (`T?`).
- **`()`**, the unit type -- truly nothing. The result of an expression that was never going to hand back a value.
- **`!`**, the never type -- a dead end. The "result" of an expression after which no further code can run.

`null` belongs to [Optionals](./options.md); this page covers the other two.

## Unit -- `()`

Any expression that doesn't produce a meaningful value evaluates to the **unit type**, written `()`. An empty block, a `for` loop, a function with no return -- they all yield `()`.

```mimas
let a: () = {};
let b: () = loop { break; };
```

## Never -- `!`

The **never type**, written `!`, is the type of an expression that diverges -- one after which no code can run. It arises from `return`, `panic`, `todo`, and infinitely-running `loop {}`. Only the compiler can produce a `!`; you can never annotate a binding with it directly.

```mimas
let a = loop {};       // `a` is `!` -- the loop never ends, so `a` is never assigned
panic("oh no!");       // `panic` diverges, so it is `!`
```

The useful part is that `!` **coerces into any type**. Because a diverging branch can never actually supply a value, the compiler lets it stand in for whatever type the surrounding code expects. That's why one arm of an `if` can bail out while the other still determines the type:

```mimas
let a: int = if some_condition {
    0
} else {
    return;  // `return` is `!`, so it fits where an `int` is expected
};
```

The same applies to `match`: an arm that `panic`s contributes `!`, which folds into the common type, so it doesn't force the other arms to become optional.
