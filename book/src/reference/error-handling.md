# Results & Error Handling

mimas has two tiers of failure: an unrecoverable **panic** that tears down the VM, and a recoverable **result** that a caller can inspect and handle.

## Panicking

The blunt instrument is `panic`. It immediately halts the VM with a message -- use it for "this should never happen" situations.

```mimas
panic("unreachable state");
```

`todo` is `panic`'s cousin for unfinished code; it halts the same way and even lets you omit the message.

```mimas
fn not_done_yet() -> int {
    todo("implement me")
}
```

Since both diverge, they have the [never type](./special-types.md) and slot into any expression -- handy as a placeholder branch.

## Results

For failure a caller can actually deal with, a function returns a **result**, written by suffixing its return type with `!`. Inside such a function, `raise` produces an error.

```mimas
fn parse_port(text: str) -> int! { // the `!` lets this function `raise`
    let n = text.to_int();
    if n == null {
        raise "port must be a number";
    }
    n!
}
```

A few rules govern results:

- `raise` is only legal inside a function whose return type is a result (`T!`). The value you raise must be a `str` -- the only error type in `0.1.0`.
- A function returning `T!` can hand back a bare `T`; it's automatically wrapped as the success case. (That's why `n!` above -- an `int` -- is a valid `int!` return.)
- `raise` itself has the [never type](./special-types.md), so it composes inside `if` and `match` arms without disturbing their result type.

```admonish todo title="`str` errors today, typed errors later"
In `0.1.0`, the error carried by a result is always a `str`. The plan is to move to an `Error` [pact](./pacts.md) you can implement for your own types, so failures can carry structured data. For now, a descriptive message is the tool.
```

## Handling a result

A result has to be dealt with before you can use the value inside it. There are two ways.

### Unwrap with `!`

The postfix `!` -- the same operator that unwraps an [option](./options.md#unwrapping) -- pulls the success value out of a result. If the result turned out to be a raised error, that becomes a runtime error and execution stops.

```mimas
let port: int = parse_port("8080")!; // 8080 -- or a hard stop if it had raised
```

```admonish warning title="`!` does not propagate -- it unwraps or dies"
Coming from Rust, the `!` here is *not* the `?` operator. It doesn't bubble an error up to your caller; it asserts success and faults the VM if it's wrong. To actually *handle* a failure and keep running, use `absolve`.
```

### Recover with `absolve`

`absolve` consumes a result and guarantees a plain value. You give it a closure that receives the error string; if the result raised, your closure runs and its value is used instead.

```mimas
let port: int = parse_port(input) absolve |err| {
    print(f"bad config: {err}");
    8080 // fall back to a default
};
```

The closure's return type must match the result's inner type -- `absolve` on an `int!` has to produce an `int` -- so the overall expression is a guaranteed, non-failing value.
