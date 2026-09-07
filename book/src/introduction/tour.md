# Tour

This page covers the entire language with brief examples for those who want a quick tour. Links to the full reference are left at each section.

## Variables

*See [Variables & Constants](../reference/variables.md).*

```mimas
let x = 0;           // inferred as int
let y: float = 1.5;  // or annotate the type explicitly
x = 10;              // `let` bindings are reassignable
const MAX = 100;     // `const` is not, and must be compile-time known

let x = "a str";     // shadowing: reuse a name, even with a new type
```

There is no `mut` and no borrow checker -- a managed runtime handles memory, so the binding model is just `let` (reassignable) and `const` (not).

## Primitive types

*See [Primitive Types](../reference/basic-types.md).*

```mimas
let i: int   = 1_000;       // 64-bit signed; underscores ignored; 0xff hex too
let f: float = 3.14;        // 64-bit IEEE-754
let b: bool  = true;
let s: str   = "hi";        // no char type; a single character is a 1-char str

// int + float promotes to float; there is no `as` cast -- use methods
let g: float = i.to_float();
let back: int = g.to_int(); // truncates toward zero
```

Integer arithmetic is **checked**: overflow is a runtime error, never a silent wrap.

## Operators

*See [Operators](../reference/operations.md).*

```mimas
// basic math
let foo = 5 + 4 - 3 * 2;
let promotes: float = 5 / 2; // / always yields a float
let divs: int = 5 ~/ 2;      // ~/ here results in 2, truncating
let rem: int = 5 % 2;        // % is the remainder 

// strs can also be added
let foob = "foo" + "bar";    // -> "foobar"

// logicals, which short-circuit
let a = true || false;
let b = true && a;
let c = !b;

// evaluations
let eq = 0 != 1 && 1 == 1;
let ltgt = 0 < 1 && 1 <= 1 && 1 >= 0 && 1 > 0;

// coalesce
let good: int = null ?? 0;

// bit operators
let and = a & b;
let or = a | b;
let xor = a ^ b;
let shift_right = 1 >> 1;
let shift_left = 1 << 1;

// Finally, every binary op has a compound form
let count = 10;
count += 1;
count ~/= 2;
// etc
```

## Strings

*See [Primitive Types](../reference/basic-types.md#strings).*

```mimas
let multi = "spans
multiple lines";                     // the newline becomes part of the value

let name = "world";
let msg = f"hi {name}, 1+1={1 + 1}"; // f-string interpolates any expression
let lit = f"{{literal braces}}";     // -> "{literal braces}"
```

## Collections

*See [Collections](../reference/collections.md).*

```mimas
// Array -- ordered, growable, single element type: [T]
let xs: [int] = [3, 1, 2];
xs.push(4);
let first = xs[0]; // 0-based, reading past the end faults at runtime

// Dictionary -- hash map of str keys to one value type: ~{V}
let scores: ~{int} = ~{ alice = 10, bob = 7 }; // keys are bare idents
let a: int? = scores["alice"]; // indexing yields V? (missing key -> null)
scores.insert("cy", 3);

// Tuple -- fixed-length, heterogeneous, indexed by position
let pair: (int, str) = (1, "one");
let one = pair.1;    // -> "one"
let (x, y) = (3, 4); // destructuring let

// `in` tests membership
0 in xs;            // array: contains the element
"alice" in scores;  // dict: has the key
"ell" in "hello";   // str: substring
```

## Control flow

*See [Control Flow](../reference/control-flow.md).*

Everything here is an **expression** -- `if`, `match`, blocks, and loops all yield values. Block braces are optional around a single expression.

```mimas
if x == 0 {
    print("zero");
} else if x > 0 {
    print("positive");
} else {
    print("negative");
}

let sign = if x >= 0 1 else -1; // as an expression, braces optional
// an `if` with no `else` must yield () -- its body can't produce a value

let total = {     // blocks yield their final semicolon-free expression
    let a = 3;
    a + 5         // -> 8
};
```

```mimas
// match -- first arm to fit wins. exhaustiveness is checked (Maranget-style).
// the below is to illustrate all patterns (and as such wouldn't compile).
let label = match value {
    0 => "zero",                  // literal
    1 | 2 | 3 => "small",         // or-pattern (multiple literals)
    n if n > 100 => "big",        // guard
    found? => found.name,         // null-bind: binds when value isn't null
    Point { x = 0, y } => f"{y}", // struct destructure (fields use `=`)
    Shape::Circle(r) => area(r),  // enum-variant destructure
    (a, b) => a + b,              // tuple destructure
    other => fallback(other),     // bare ident binds anything; `_` too
};

let code = match cmd {            // end with a bare `!` to promise exhaustiveness;
    "go" => 1,                    // reaching it is a runtime fault
    "stop" => 2,
    !
};
```

```mimas
loop {                            // infinite until `break`
    if done() { break; }
    continue;
}
let answer = loop { break 42; }; // `loop` can `break value` -> 42

while ready() { work(); }
for item in xs { print(item); }  // iterates arrays, dicts, str, 0..n on int

// `collect` turns any loop into an array builder -- mimas's comprehension
let doubled = for n in xs collect n * 2;
let evens = for n in xs { 
    if n % 2 == 0 collect n;     // filter with `if`
}; 
```

## Options

*See [Options](../reference/options.md).*

A value that may be `null` is an **option**, written `T?`. `null` exists only where a `?` invites it, so the compiler guarantees you never hit an unexpected null.

```mimas
let maybe: int? = lookup(); // an int, or null
// let bad = maybe + 1;     // compile error: maybe may be null

let n = maybe ?? 0;         // ??  fallback when null
let len = name?.len();      // ?.  short-circuits to null on a null receiver
let cell = grid[1]?[0];     // ?[] the same, for indexing
let sure = maybe!;          // !   asserts non-null (faults if it was null)

let n? = maybe else return; // let/else: bind, or diverge and move on
```

`T??` automatically flattens to `T?` -- there is no option-of-an-option.

## Results & errors

*See [Results & Error Handling](../reference/error-handling.md).*

Two tiers of failure: an unrecoverable **panic** that halts the VM, and a recoverable **result** (`T!`) a caller can handle.

```mimas
panic("unreachable");           // halts the VM immediately
todo("later");                  // panic's cousin for unfinished code (msg optional)

fn parse_port(s: str) -> int! { // `!` return type lets the fn `raise`
    let n = s.to_int();         // str.to_int() -> int?
    if n == null {
        raise "not a number";   // the raised value must be a str*
    }
    n!                          // a bare T auto-wraps as the Ok case
}

// unwrap with `!`: take the value, or fault hard if it raised
let p: int = parse_port("80")!;

// recover with `absolve`: handle the error string, keep running
let q: int = parse_port(input) absolve |e| {
    print(f"bad: {e}");
    8080
};
```

`panic`, `todo`, `return`, `raise`, and an endless `loop {}` are all **never** (`!`) typed, so they slot into any branch without disturbing its type.

```admonish todo title="*typed errors are coming"
In `0.1.0` the error carried by a result is always a `str`. The plan is to move to an `Error` [pact](../reference/pacts.md) you can implement for your own types, so failures can carry structured data. For now, a descriptive message is the tool.
```

## Functions & closures

*See [Functions & Closures](../reference/functions.md).*

```mimas
fn square(n: int) -> int { n * n }   // body is a block; last expr returns
fn greet(name: str) { print(name); } // no `->`: returns ()

// optional params (with const-foldable defaults) follow the required ones
fn connect(host: str, port=8080) {}
connect("localhost");                // port defaults to 8080
connect("localhost", port=9090);     // name an optional to skip earlier ones

// closures: inline, anonymous, pipe-delimited. return type optionally inferred
let add = |a: int, b: int| a + b;
let now = || current_time();         // no args
```

`fn`s live only at the file's top level and capture nothing -- they're *second-class* (passable as arguments, but not bindable). Closures are *first-class* (bindable, storable) and capture their surrounding scope.

```mimas
let g = add;       // ok -- closures are values
// let h = square; // error -- a fn isn't a value you can bind
```

## Structs

*See [Structs](../reference/types/structs.md).*

```mimas
struct Player { name: str, score: int }

let p = Player { 
    name = "ada",                // construct with `=`
    score = 0 
}; 
print(p.name);                   // read fields with `.`

impl Player {                    // a type may have many impl blocks
    const MAX = 100;             // associated const
    fn new(name: str) -> Self {  // associated fn (no self)
        Self { 
            name = name, 
            score = 0 
        }
    }
    fn won(self) -> bool {       // method (takes self)
        self.score >= Self::MAX
    }
}

let p = Player::new("ada");      // `::` reaches associated items
p.won();                         // `.` calls a method (= Player::won(p))

struct Vec2(float, float)        // tuple struct: positional fields, indexed
let v = Vec2(1.0, 2.0);
let vx = v.0;
struct Marker;                   // field-free marker type
```

## Enums

*See [Enums](../reference/types/enums.md).*

```mimas
enum Shape {
    Circle(float),         // tuple variant
    Rect(float, float),
    Labeled { text: str }, // struct variant
    Empty,                 // payload-free
}

let c = Shape::Circle(1.0);
let e = Shape::Empty;

impl Shape {
    fn area(self) -> float {
        match self {            // every variant must be covered
            Shape::Circle(r) => 3.14159 * r * r,
            Shape::Rect(w, h) => w * h,
            _ => 0.0,
        }
    }
}
```

## Pacts

*See [Pacts](../reference/pacts.md).*

A **pact** is mimas's trait analogue: a named set of method, associated-function, and constant signatures a type satisfies with `impl Pact for Type`. It's how you abstract over types without generics.

```mimas
pact Draw {
    fn draw(self);
    fn describe() { print("a shape"); } // items may ship a default
}

struct Square;
impl Draw for Square {
    fn draw(self) {}                    // `describe` is inherited
}

fn render_all(items: [Draw]) {          // a pact stands in anywhere a type is expected
    for item in items { item.draw(); }
}

struct Widget {
    item: Named + Identified,           // require several pacts at once with `+`
}
```

## Modules & scripts

*See [Modules](../reference/modules.md) and [Scripts](../reference/scripts.md).*

A plain file is a **script** -- it runs top to bottom, no `main` required. A file that opens with `module` is a **library**: it organizes items and can't run top-level code.

```mimas
// colors.mim
module @; // name the module after the file (or: `module graphics::colors;`)

const INTERNAL = 0;          // private by default
pub const RED = 0xff0000;    // `pub` exposes it across modules
pub fn mix(a: int, b: int) -> int {
    // elided
    a
}
```

```mimas
// main.mim -- a script
use colors;               // reach items via colors::RED
use colors::{ RED, mix }; // or pull names in directly (also `colors::*`)

print(mix(RED, 0x00ff00));
```

## Extending with Rust

*See [Extension with Rust](../extension-with-rust.md).*

mimas is built to embed: share Rust types and functions with the `#[mimas]` macro and they're type-checked like native ones.

```rust
#[mimas]
struct User(String);

#[mimas]
impl User {
    fn greet(self) { println!("Hello, {}!", self.0); }
}
```

```mimas
let user = User("mimas");
user.greet(); // -> Hello, mimas!
```
