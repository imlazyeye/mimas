# Pacts

mimas is built for embedded use, so Rust's full trait machinery -- generics, associated types, blanket impls, and the constraint solver behind them -- is intentionally out of scope. Even so, abstracting over types is sometimes genuinely useful: without any way to do it, every function that wants to handle "anything with this behavior" has to be rewritten per concrete type.

It comes up most at the FFI line. A host project large enough to span several crates often leans on traits at its boundaries, and a mimas script needs some way to interoperate with at least the simplest of those patterns.

A **pact** fills that role. It's a named set of method, associated-function, and constant signatures -- a contract a type satisfies by writing `impl PactName for TheType { ... }` and supplying the listed items. Anywhere a type is expected, a pact can stand in, and the checker will accept any value whose type fulfills it.

```mimas
pact Draw {
    fn draw(self);
}

struct Square;
impl Draw for Square {
    fn draw(self) { print("a square"); }
}

struct Circle;
impl Draw for Circle {
    fn draw(self) { print("a circle"); }
}

fn render_all(items: [Draw]) {
    for item in items {
        item.draw();
    }
}

render_all([Square, Circle]);
```

A collection literal holding several types settles on the pacts they share, which is how you build "a list of things that implement `Draw`" without generics. If the elements have no pact in common, it's an ordinary type mismatch.

## Constants

A pact can require constants as well as methods.

```mimas
pact Named {
    const NAME: str;
}

struct Square;
impl Named for Square {
    const NAME = "Square";
}

let n = Square::NAME; // "Square"
```

Constants are read off a **concrete type**. Unlike methods, they don't dispatch -- see [Limitations](#limitations) below.

## Default implementations

A pact method can ship with a default body, which an implementer may use as-is or override.

```mimas
pact Greet {
    fn name(self) -> str;

    fn hello(self) -> str {
        f"hi, {self.name()}"
    }
}

struct Dog { tag: str }
impl Greet for Dog {
    fn name(self) -> str { self.tag }
    // `hello` is inherited
}

print(Dog { tag = "rex" }.hello()); // "hi, rex"
```

A default body can call the pact's other methods through `self`, including ones the implementer supplies. An impl that defines the method itself overrides the default.

## Reaching pact methods through a value

Without generics, mimas can't lean on Rust's `T::ITEM` syntax to reach an item on a constrained type. This is why [dot access reaches associated items](./types/structs.md#reaching-items-through-a-value) -- given a value known only by its pact, `.` is how you get at its methods.

```mimas
fn announce(thing: Greet) {
    print(thing.hello());
}
```

## Binding multiple pacts

An annotation can require several pacts at once with `+`:

```mimas
struct Widget {
    item: Named + Greet,
}
```

To make a multi-pact bound optional or a result, wrap it in parentheses first so the `?`/`!` applies to the whole thing:

```mimas
struct Widget {
    item: (Named + Greet)?,
}
```

## Limitations

Pacts are the youngest part of the language, and 0.1.0 ships a deliberately small slice of them. Each of the following is rejected with a clear error rather than silently misbehaving.

**Constants don't dispatch.** A pact constant can only be read off a concrete type. Reaching one through a pact-typed value has no answer at runtime -- every impl declares its own value, and the receiver's concrete type isn't known:

```mimas
fn tag_of(thing: Named) -> str {
    thing.NAME // error: pact constant `NAME` can't be reached through `Named`
}
```

The workaround is to require a method instead, which does dispatch:

```mimas
pact Named {
    fn name(self) -> str;
}
```

**`Self::` doesn't resolve inside a default body.** A default body is compiled once and shared by every implementer, so `Self` there is still the abstract pact rather than a concrete type. `Self::CONST` and `Self::assoc_fn()` are both rejected inside one. Lowercase `self` -- fields, methods, parameters -- works normally, and `Self::` resolves as expected inside an ordinary `impl` block.

**No generics.** A pact bound is the only form of abstraction over types; there are no type parameters, associated types, or blanket impls, and none are planned for 0.1.0.
