# Control Flow

mimas's control-flow constructs -- `if`, `match`, and the loops -- are all **expressions**: each one evaluates to a value, not just steer execution. That property runs through everything in this section:

```mimas
let label = if score >= 50 { "pass" } else { "fail" };

let kind = match tag {
    0 => "circle",
    _ => "other",
};

let first_even = for n in numbers { if n % 2 == 0 { break n; } };
```

The pages here cover [`if` & `if let`](./control-flow/if-else.md), [`match`](./control-flow/match.md), the [`loop`](./control-flow/loops.md) / [`while`](./control-flow/while.md) / [`for`](./control-flow/for-in.md) family, and the [`collect`](./control-flow/collection.md) operator for building arrays out of a loop.
