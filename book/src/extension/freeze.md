# Freeze

[Fixtures](./fixtures-and-context.md) must own their data. The Vm holds them for its whole life, so everything inside must be `'static`. This is directly against a very common pattern for a scripting layer: getting mutable access to the wider program in the middle of a frame. That's what the `Freeze` trait is for. 

A `FreezeCell` is an empty slot that holds a borrow for the duration of a single closure and takes it back the moment that closure ends. While the borrow is in the cell, natives can reach it. The instant your frame is over, it's gone.

Let's wire up an example similar to what we did on the `Fixtures` page, only this time, let's say that our script needs to not just _read_ information from the host but _write_ it too.

```rust
// rust
use mimas::vm::freeze::{Freeze, FreezeCell};

struct Database(Vec<String>);

type DatabaseCell = FreezeCell<Freeze![&'freeze mut Database]>;
```

`Freeze![&'freeze mut Database]` reads as "this cell holds a `&mut Database` for some lifetime we'll call `'freeze`." The cell itself is `'static` and starts empty, which is exactly what makes it a legal fixture.

```rust
// rust
fn main() {
    let mut database = Database(Vec::new());
    let mut vm = mimas::compile_source(SOURCE).unwrap();
    vm.run().unwrap();

    let cell = vm.fixture::<DatabaseCell>();
    cell.freeze(&mut database, || vm.call_fn("call_me").unwrap());

    assert_eq!(database.0.first(), Some(&"hello!".to_string()));
}
```

Now any native can add to the database, and our script can use it like anything else:

```rust
// rust
#[mimas]
fn add_entry(ctx: Ctx, entry: String) {
    ctx.fixture::<DatabaseCell>()
        .with_mut(|database| {
            database.0.push(entry);
        })
        .unwrap();
}
```

```mimas
// mimas
fn call_me() {
    add_entry("hello!");
}
```

A few things worth noticing:

- The fixture is the *cell*, not the `Database`. The `Database` never enters the Vm and never needs `Default` -- the cell is built empty on first access, which is exactly the state it should be in between frames.
- `with_mut` returns a `Result` because the cell might be empty: a native that runs outside any freeze scope gets an error instead of stale data. `add_entry` just `unwrap`s it, which is a plain panic of our own choosing; if some of your natives can legitimately run outside a frame, surface a [runtime error](./runtime-errors.md) instead.
- `vm.fixture` hands back a handle that is deliberately *not* borrow-tied to the Vm -- that's what lets us hold `cell` while calling `vm.call_fn` (a `&mut` borrow) inside the closure. The cost of that freedom is one rule: drop the handle before the Vm. Declaring it after the `vm`, like above, gets you that for free.
- Need to lend more than one thing per frame? Cells nest: `a.freeze(x, || b.freeze(y, || ...))`.

```admonish info
The freeze pattern comes from Catherine West: it originates in [piccolo](https://github.com/kyren/piccolo)'s `freeze` module, and our implementation follows the slimmer form another of her projects, [fabricator](https://github.com/kyren/fabricator).
```

A complete, runnable example of this pattern lives at [examples/game-loop](https://github.com/imlazyeye/mimas/tree/main/examples/game-loop).
