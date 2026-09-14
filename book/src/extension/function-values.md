# Function Values

Scripts are able to pass their own functions or closures back to the host. The host protects it from garbage collection while its held, and is able to call it at will (ex: for a callback system).

Functions and closures can be passed to the host like any other value. Generally, its easiest to do so with `anon::T`.

Calling `signature` on a mimas function hands back the `FnHeader` that the compiler worked out for it (the parameters with their types and defaults as well as the return type). The header is borrowed straight out of the loaded program rather than copied, so a check like the one below costs nothing.

```rust
// rust
use std::cell::RefCell;

use mimas::{
    Ctx, Val, mimas,
    vm::{RtErr, Stashed, anon},
};

/// Functions the script has handed over, waiting on the host to run them.
#[derive(Default)]
struct Pending(RefCell<Vec<Stashed>>);

#[mimas]
fn execute_call<'gc>(ctx: Ctx<'gc>, f: anon::T<'gc>) -> Result<(), RtErr> {
    let Some(header) = f.0.signature(ctx) else {
        return Err(RtErr::InvalidArgument(
            "`execute_call` takes a function".into(),
        ));
    };
    ctx.fixture::<Pending>().0.borrow_mut().push(ctx.stash(f.0));
    Ok(())
}
```

A native can't run the function itself. It holds a `Ctx`, not the Vm, and the Vm is already mid-dispatch on the native. Taking a function value always means handing it to the host, which runs it once it has control back.

## Holding onto one

`ctx.stash` turns a value into a `Stashed`: a handle with no `'gc` lifetime, so the host can keep it in a [fixture](./fixtures-and-context.md), a struct field, or anything else you like. It survives collection, and keeps the value alive for as long as the handle is around. Dropping it lets the value be collected again.

A handle belongs to the Vm that made it. You cannot use a stashed value on a Vm other than the one who made it, nor on a later recompile on said Vm.

## Calling it back

`vm.call_value` runs a stashed function and converts what it returned, the same way `vm.call` does for one looked up by name.

```rust
// rust
let mut vm = mimas::compile_source(SOURCE).unwrap();
vm.run().unwrap();

let pending: Vec<Stashed> = vm.fixture::<Pending>().0.borrow_mut().drain(..).collect();
for f in &pending {
    vm.call_value::<()>(f, ()).unwrap();
}
```

A few things worth noticing:

- The script hands its functions over when its top-level code runs, which is why `vm.run()` comes first. Draining the fixture into a `Vec` keeps it unborrowed while the Vm runs.
- Arguments work as they do for `vm.call`: a tuple, with any trailing parameter you leave off filled from the function's own default.
- `vm.call_value_then` is the `then` variant, mirroring `vm.call_then`: it takes a closure run with the arena still open, for reading whatever a native put aside during the call.
- A fault leaves the Vm usable. The frames unwind to where they were, so the next call starts clean.

## Passing script values as arguments

Everything so far called a function that needed nothing from the host. A method needs one more thing: something to call it on.

```rust
// rust
/// The method the script wants run each tick, and the value to run it on.
#[derive(Default)]
struct Tick(RefCell<Option<(Stashed, Stashed)>>);

#[mimas]
fn on_tick<'gc>(ctx: Ctx<'gc>, f: anon::T<'gc>, receiver: anon::U<'gc>) {
    *ctx.fixture::<Tick>().0.borrow_mut() = Some((ctx.stash(f.0), ctx.stash(receiver.0)));
}
```

The two parameters take separate `anon` slots. Every `anon::T` in one signature is the *same* type, so a second `anon::T` would tie the receiver to the function's own type.

```mimas
struct Tally { n: int }
impl Tally { 
    fn bump(self, by: int) -> int { 
        self.n += by; self.n 
    } 
}

let tally = Tally { n = 1 };
on_tick(Tally::bump, tally);
```

A `&Stashed` is an argument type, so the receiver goes back in as the first argument:

```rust
// rust
let (f, receiver) = vm.fixture::<Tick>().0.borrow_mut().take().unwrap();
assert_eq!(vm.call_value::<i64>(&f, (&receiver, 2)).unwrap(), 3);
```

`self` is nothing special here: it's parameter 0 of `bump`'s header, so `(&receiver, 2)` fills it exactly like any other argument. Two parameters on the header, one argument at the script's call site.

````admonish tip title="Usually a closure is simpler"
When the host has nothing to add to the call, let the script capture the receiver and hand over one closure instead:

```mimas
execute_call(|| { tally.bump(2); });
```

That leaves one handle to keep and nothing to pair up. Reach for the method and receiver shape when the host supplies the arguments, or when it calls the same method on several different values.
````

## Giving one back to the script

`ctx.fetch` reads a stashed value back as a `Val` anywhere you have a `Ctx`. A native can return one, which lets a script get a function out of the host and run it itself:

```rust
// rust
#[mimas]
fn handler<'gc>(ctx: Ctx<'gc>) -> Val<'gc> {
    match ctx.fixture::<Pending>().0.borrow().first() {
        Some(f) => ctx.fetch(f),
        None => Val::Null,
    }
}
```

```mimas
let f = handler();
f();
```

```admonish warning title="The compiler can't check that call"
What the script gets back is an untyped value, so nothing checks the call where it's written -- this is the [`Val` hazard](./working-with-types.md#admonition-tread-carefully-with-val). A mistake that would normally be a compile error becomes a runtime fault: calling with the wrong number of arguments, or calling something that isn't a function at all, raises an error pointing at the call rather than being caught up front. Utilize the information in the `FnHeader` to handle the error case.
```

````admonish warning title="A stashed closure keeps its captures"
A closure holds everything it captured, and stashing it extends that for the life of the handle. A closure that captured a large array keeps that array out of the collector's hands until you drop the `Stashed`.

```mimas
let rows = load_everything(); // stays alive as long as the handle does
execute_call(|| print(rows[0]));
```
````
