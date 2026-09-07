# Arrays

An array is an ordered, growable sequence of values that all share one type -- like a `list` in Python or a `Vec` in Rust. Its type is written `[T]`.

```mimas
let xs: [int] = [0, 1, 2, 4];
let first = xs[0]; // 0 -- index with []
```

Indexing is zero-based, and reading past the end is a runtime error:

```mimas
let xs = [1, 2, 3];
let oops = xs[9]; // runtime error: index out of bounds
```

Arrays carry a set of built-in methods for inspecting and growing them, such as `len`, `push`, and `contains`:

```mimas
let xs = [3, 1, 2];
xs.push(4);       // xs is now [3, 1, 2, 4]
let n = xs.len(); // 4
```

To build a *new* array by transforming or filtering an existing one, use a [`collect`](../control-flow/collection.md) loop:


```mimas
let doubled = for x in [1, 2, 3] collect x * 2; // [2, 4, 6]
```
