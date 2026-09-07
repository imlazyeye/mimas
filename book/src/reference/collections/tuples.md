# Tuples

A tuple is a fixed-length, **heterogeneous** sequence: an anonymous product type where each position can hold a different type. Tuple types are written as a parenthesized list.

```mimas
let pair: (int, int) = (0, 1);
let mixed: (float, str) = (0.1, "hello!");
```

Reach into a tuple by position with dot-and-index:

```mimas
let mixed = (0.1, "hello!");
let x: float = mixed.0; // 0.1
let s: str   = mixed.1; // "hello!"
```

Because each position can be a different type, a tuple is *not* iterable -- a single loop binding would have no consistent type to take. That's why index access exists. To pull a tuple apart in one step, match on it:

```mimas
let point = (3, 4);
let sum = match point {
    (a, b) => a + b,
};
// -> 7
```

You can also destructure a tuple straight into a `let`, binding each position at once:

```mimas
let (x, y) = (3, 4); // x is 3, y is 4
```

```admonish note title="Tuple vs. tuple struct"
A tuple is anonymous and structural: `(int, int)` is just "two ints." When you want that shape to carry a *name* and an identity -- a `Vec2` that isn't interchangeable with every other `(float, float)` -- reach for a [tuple struct](../types/structs.md#tuple-structs).
```
