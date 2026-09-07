# Fixtures & Context

All `mimas` tagged functions have access to `Ctx`, a handle into the current execution. To use it, declare it as the first argument of your function.

```rs
use vm::Ctx;

#[mimas]
fn with_ctx(ctx: Ctx) {
    // makes a new array within the GC
    let _array = ctx.new_array(vec![]);

    // intern a string to the GC
    ctx.intern("foobar");
}
```

Because of the automatic [marshalling](./working-with-types.md#the-conversion-table) of values it's rare to need access to `Ctx` for the purposes shown above. You're more likely to use it for accessing **fixtures**: per-Vm storage that lets your mimas code reach important parts of your program.

Consider a scenario where mimas needs access to data that is collected at bootup of its host program.

```rust
// rust
#[mimas]
struct Request(String);

fn main() {
    let user_request = Request(std::env::args().nth(1).unwrap());
    let vm = mimas::compile_source(some_source).unwrap();
}

#[mimas]
fn process_request() {
    // how do I get that request?
}
```

To achieve this we can use a fixture: a value the Vm owns that both sides -- the host outside, and your native functions inside -- can reach.

Fixtures have two rules that shape how you use them:

1. **You never hand the Vm a value.** A fixture is built *by* the Vm, in place, via `Default`, the first time either side asks for it. There is no insert or replace -- just `vm.fixture::<T>()` from the host and `ctx.fixture::<T>()` from a native, both returning a reference to the same instance.
2. **You only ever get a shared reference.** To put data *into* a fixture after the fact, the fixture type itself provides interior mutability -- a `RefCell` and a setter method.

Which means our `Request` doesn't become the fixture directly; instead we give the Vm a *slot* that holds one:

```rust
// rust
use std::cell::RefCell;

#[mimas]
#[derive(Clone)]
struct Request(String);

#[derive(Default)]
struct CurrentRequest(RefCell<Option<Request>>);

impl CurrentRequest {
    fn set(&self, request: Request) {
        *self.0.borrow_mut() = Some(request);
    }

    fn get(&self) -> Option<Request> {
        self.0.borrow().clone()
    }
}

fn main() {
    let user_request = Request(std::env::args().nth(1).unwrap());
    let mut vm = mimas::compile_source(some_source).unwrap();

    // first access creates the (empty) fixture; `set` fills it
    vm.fixture::<CurrentRequest>().set(user_request);
    vm.run().unwrap();
}

#[mimas]
fn process_request(ctx: Ctx) -> Option<Request> {
    // here it is!
    ctx.fixture::<CurrentRequest>().get()
}
```

```mimas
// mimas
let request = process_request()!;
print(request);
```

A few things worth noticing:

- `vm.fixture` and `ctx.fixture` reach the *same* slot -- the host fills it before `run`, the native reads it during.
- `process_request` returns `Option<Request>`, which mimas sees as `Request?` -- the script decides what to do when no request was installed (here it just unwraps with `!`).
- This is exactly how the standard library's `std::sys::arg` works: the CLI fills a `ScriptArgs` fixture at startup, and the native reads it back per call.

```admonish info title="What about &mut state?"
A fixture must be `'static` -- it can hold owned data, but not a borrow of your program's state. Lending *borrows* into the Vm for the duration of a `run` requires something else: a `FreezeCell` fixture. That's what comes [next](freeze.md)!
```
