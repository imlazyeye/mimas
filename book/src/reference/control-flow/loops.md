# Loop

`loop` is the simplest loop: it repeats its body forever until something stops it. Use `break` to exit and `continue` to skip to the next iteration.

```mimas
loop {
    if done() {
        break;     // leave the loop
    } else {
        continue;  // jump straight to the next pass
    }
}
```

## Loops are expressions

A `loop` evaluates to whatever value you `break` with, which makes it a clean way to retry until you get a result:

```mimas
let answer: int = loop {
    let guess = next_guess();
    if is_valid(guess) {
        break guess; // the loop evaluates to this
    }
};
```

The result type depends on how the loop ends:

| The loop ends with | evaluates to |
| :--- | :--- |
| `break value` | the value's type, e.g. `int` |
| `break` with no value | `()` |
| never breaks (`loop {}`) | [`!`](../special-types.md), the never type |

```admonish note title="`break value` is for `loop` only"
Plain `loop` is the one form guaranteed to run, so it can hand back a `T` directly. [`while`](./while.md) and [`for`](./for-in.md) might never enter their body, so a value they break out becomes a `T?` instead -- see those pages.
```
