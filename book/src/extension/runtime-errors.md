# Runtime Errors

There are three different methods of raising errors at runtime. The first one is just to `panic`, which doesn't really count, so we'll skip past that one!

## Unrecoverable errors

Using an ordinary `Result` as the return type of your function will raise the error immediatly within the VM if the error case is hit. 

```rs
// rust
#[mimas]
fn oops() -> Result<(), RtErr> {
    Err(RtErr::Custom("oops!"));
}
```
```mimas
// mimas
oops(); // runtime error: oops!
```

The standard library avoids using these in favor of relying on the type system or handing back raisable errors, but hey, you do you.

## Raisable errors

To return an actual _mimas_ Result, you use `Raisable`.

```rs
// rust
#[mimas]
fn test(b: bool) -> Raisable<()> {
    if b {
        Raisable::Ok(())
    } else {
        Raisable::Raised("not true!");
    }
}
```
```mimas
// mimas
test(false) absolve |e| print(e); // > not true!
```

`Raisable` has a blanket implementation on `Result<T, E>` (so long as `E` is `Display`).

```rs
#[mimas]
fn load_file(path: &str) -> Raisable<String> {
    std::fs::read_to_string(path).into()
}
```