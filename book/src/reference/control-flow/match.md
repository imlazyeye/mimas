# Match

A `match` compares a value against a series of patterns and runs the first arm that fits. Each arm is `pattern => expression`, and arms are separated by commas.

```mimas
match status {
    0 => print("idle"),
    1 => print("running"),
    2 => print("done"),
    _ => print("unknown"),
}
```

Like everything else in this section, `match` is an expression -- every arm yields a value, and the whole `match` evaluates to it:

```mimas
let label = match status {
    0 => "idle",
    1 => "running",
    _ => "unknown",
};
```

## Patterns

mimas supports a wide range of patterns:

```mimas
match value {
    0 => "zero",                 // literal
    1 | 2 | 3 => "small",        // multiple literals (an "or" pattern)
    n if n > 100 => "huge",      // a guard -- an extra boolean condition
    found? => found.label,       // null-bind: matches & binds when `value` is not null
    Point { x = 0, y } => y,     // struct destructure (note: `=`, like construction)
    Shape::Circle(r) => area(r), // enum-variant destructure
    (a, b) => a + b,             // tuple destructure
    other => fallback(other),    // a bare name binds anything (the catch-all)
}
```

A few things to keep in mind:

- **Struct and variant patterns mirror construction.** Fields use `=` (`Point { x = 0 }`), not `:`. Write just the field name to bind it (`Point { x, y }` binds both `x` and `y`).
- **A bare identifier matches anything** and binds the value to that name -- this is your wildcard / default arm. `_` works too when you don't need the binding.
- **Guards** (`n if cond`) add a runtime condition to an arm.

## Exhaustiveness

mimas checks matches for exhaustiveness using the same Maranget-style analysis as Rust: the compiler proves that every possible value is covered.

```mimas
enum Flag { On, Off }

match flag {
    Flag::On => {},
    // compile error: non-exhaustive match -- missing `Flag::Off`
}
```

Types with a finite shape -- `bool`, enums, tuples, single-variant structs, options -- can be exhausted by listing their cases. Open-ended types like `int`, `float`, and `str` can't, so they always need a catch-all.

You have two ways to deliberately *not* enumerate every case.

### A catch-all arm

A bare identifier (or `_`) at the end soaks up everything that's left, which satisfies the exhaustiveness check:

```mimas
match flag {
    Flag::On => do_thing(),
    _ => {},
}
```

### The `!` terminator

When you're certain the remaining cases can't occur, end the `match` with a bare `!`. It promises exhaustiveness and, if execution ever actually reaches it, raises a runtime error.

```mimas
let n = match command {
    "go" => 1,
    "stop" => 2,
    ! // if it's neither, we made a promise we couldn't keep -- error at runtime
};
```

Because the `!` arm has the [never type](../special-types.md), it doesn't add to the match's result type. The expression above is a plain `int`.

````admonish warning title="Guards don't count toward exhaustiveness"
An arm with a guard might not fire, so the compiler can't treat it as covering its pattern. You'll still need a catch-all even when a guarded arm "looks" total:

```mimas
match flag {
    Flag::On => {},
    Flag::Off if quiet() => {}, // doesn't fully cover `Flag::Off`
    _ => {},                    // still required
}
```
````
