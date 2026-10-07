# Variables & Constants

Local variables are introduced with `let`. A `let` binding can be reassigned.

```mimas
let a: int = 0;
let b = 1;  // type inferred as int

a = 10;     // reassignment is fine
```

```admonish tip title="Inference does the heavy lifting"
Type inference in mimas is strong. The only places you *must* write a type are function signatures and type declarations (`struct`, `enum`, `pact`). Everywhere else the compiler works it out from the value.

We still annotate freely throughout this book -- `let a: int = 0` -- purely to make each example's types obvious. You won't need most of them in real code.
```

## Reassignment

A `let` binding is reassignable, but its **type is fixed** at declaration. Assigning a value of a different type is an error -- use shadowing (below) if you genuinely want a new type.

```mimas
let count = 0;
count = 5;        // valid
count += 1;       // valid, count is now 6
count = "six";    // compile error: expected int, found str
```

## Shadowing

A new `let` may reuse a name already in scope. The new binding *shadows* the old one and can even change the type. This is a fresh variable, not a mutation of the original.

```mimas
let a: int = 0;
let a: str = "hello!"; // valid -- `a` is now a str
```

## Let else

`let`/`else` binds a [pattern](./control-flow/match.md) and runs an `else` block when the value doesn't match. The `else` has to diverge -- `return`, `break`, `panic`, and so on -- so execution only continues past it when the binding succeeded.

```mimas
# enum Shape { Circle(float), Square(float) }
# fn example(shape: Shape) {
let Shape::Circle(r) = shape else {
    return;
};
// `r` is in scope from here on
# }
# example(Shape::Circle(1.0));
```

It accepts any pattern `match` does, including the `?` null-bind for [options](./options.md):

```mimas
# fn lookup_port() -> int? { 8080 }
let port? = lookup_port() else {
    panic("no port configured");
};
```

## Unused bindings

Prefixing a name with `_` marks it as intentionally unused.

```mimas
# fn compute() -> int { 0 }
let _scratch = compute();
```

```admonish note title="No warnings yet"
We currently only have errors, no warnings, so the `_` prefix is a no-op for now -- it documents intent for when unused-variable warnings land.
```

## Constants

Constants are declared with `const` and cannot be reassigned.

```mimas
const FOO: int = 0;
FOO = 1; // compile error: constants cannot be mutated after declaration
```

A constant's value must be computable at **compile time**. Literals, operators, and references to other constants are all fair game; anything that requires running code at runtime -- like a function call -- is not.

```mimas
# fn some_call() -> int { 0 }
const BAR = 0;
const FIZZ = BAR + 1;          // valid -- built from another const
const GREETING = "hello";      // valid -- a literal

const NOPE = some_call();      // compile error: constants must be known at compile time
```

```admonish todo
Our constant folding could likely handle evaluating whether a function is fully knowable at compile time, but that will come in a future update.
```

### Enums, structs and collections

Enum variants, arrays, tuples, dicts, and structs can be constant too, as long as whatever they hold is constant as well. Building a tuple variant like `Shape::Circle(4)` looks like a function call but isn't one, so it is allowed.

```mimas
enum Shape {
    Dot,
    Circle(int),
}

const RADIUS = 4;
const DOT = Shape::Dot;
const CIRCLE = Shape::Circle(RADIUS * 2);
const SHAPES = [DOT, CIRCLE, Shape::Circle(1)];
```

A constant like this works as if its value were written out wherever its name appears. Each read gives its own value, equal to every other read but separate from it, so nothing you do to a value you read can change the constant.

```mimas
struct Point {
    x: int,
}

const ORIGIN = Point { x = 0 };

let a = ORIGIN;
let b = ORIGIN;
a.x = 5;

print(b.x);      // 0 -- `a` and `b` are two reads, so two points
print(ORIGIN.x); // 0 -- the constant never changes
```

Only reading the constant by name does this. Once a value is in a variable it is shared like any other, so `let c = a;` makes `c` and `a` the same point.

Numbers, bools, strings and `null` have none of this. They are the same value wherever they're read, and a string can't be changed in place.

### Operators

A constant can be set with operators, which are worked out at compile time. They work on numbers, strings and bools, and `==` and `!=` also work on arrays and tuples of those. Enum variants, structs and dicts can't be compared in a constant.

```mimas
# enum Shape { Dot, Circle(int) }
# const DOT = Shape::Dot;
const LIMIT = 4 * 2 + 1;
const SAME = [LIMIT, 1] == [9, 1];   // valid -- true

const NOPE = DOT == Shape::Dot;      // compile error: an enum variant can't be compared in a constant
```

This only limits what a `const` can be set to. Comparing against a constant in ordinary code, like `shape == DOT`, is fine.
