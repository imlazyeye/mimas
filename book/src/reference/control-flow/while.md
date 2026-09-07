# While

A `while` loop checks a `bool` condition before each pass and stops as soon as it fails.

```mimas
let i = 0;
while i < 5 {
    i += 1;
}
// -> i is 5
```

## Breaking a value

Because a `while` loop might never run -- its condition could be false on the very first check -- a value broken out of it is wrapped in an [option](../options.md). The loop yields `null` if it ends without a `break value`.

```mimas
let found: int? = while has_next() {
    let x = next();
    if matches(x) {
        break x; // -> int?, because the loop might not have run at all
    }
};
```

## While let

`while let` is to `while` what [`if let`](./if-else.md#if-let) is to `if`: instead of a `bool`, it evaluates an expression each pass and keeps looping as long as the result is **not `null`**, binding the unwrapped value in the body.

```mimas
while let job = next_job() {
    // runs as long as `next_job()` returns a value;
    // stops the first time it returns null
    process(job);
}
```

It's the idiomatic way to drain a source that signals "nothing left" with `null`.
