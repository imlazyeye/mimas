# For

A `for` loop walks over the elements of something iterable, binding each one in turn.

```mimas
let xs = [10, 20, 30];
for x in xs {
    print(x); // 10, then 20, then 30
}
```

## What you can iterate

| Iterable | Each binding is |
| :--- | :--- |
| `[T]` (array) | an element, `T` |
| `~{V}` (dict) | a `(str, V)` pair of key and value |
| `str` | each character, as a one-character `str` |
| `int` | the numbers `0` up to (but not including) the value |

```mimas
for pair in ~{ x = 1, y = 2 } {
    print(pair); // [x, 1], then [y, 2]
}

for c in "hi" {
    print(c); // "h", then "i"
}

for i in 3 {
    print(i); // 0, 1, 2
}
```

````admonish tip title="Need the index?"
Call `.enumerate()` on an array to pair each element with its position:

```mimas
for pair in ["a", "b"].enumerate() {
    print(pair); // [0, a], then [1, b]
}
```
````

Tuples are intentionally *not* iterable: each position can hold a different type, so a single loop binding would have no consistent type. Reach into a tuple by index instead (`t.0`, `t.1`). See [Tuples](../collections/tuples.md).

## Breaking a value

Like [`while`](./while.md), a `for` loop isn't guaranteed to run -- the collection might be empty -- so a value it breaks comes back as an [option](../options.md).

```mimas
let first_big: int? = for n in numbers {
    if n > 100 {
        break n; // -> int?, since `numbers` could be empty
    }
};
```

A `for` loop that never breaks a value evaluates to `()`. To *build* a value out of every iteration instead of breaking once, use [`collect`](./collection.md).

```admonish warning title="Mutation during iteration"
Avoid mutating a collection while you are iterating over it. mimas does not catch this today -- it has no borrow checker, and a purely-runtime check would either reject safe code or ambush you mid-loop, both of which cut against the language's goals. A compile-time check is planned. Until then, mutating the collection you are looping over can behave unexpectedly; pushing to an array inside its own `for`, for instance, may not terminate.

When the compiler can prove a loop does not change the length of what it iterates, it computes that length once instead of on every pass. This is only an optimization, and the cost of missing it is negligible unless you run many thousands of iterations. If you do need to grow a collection as you walk it, iterate a copy or use a `loop` with the bounds handled yourself.
```