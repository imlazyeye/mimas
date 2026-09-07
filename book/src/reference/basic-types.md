# Primitive Types

mimas has four primitive types: `int`, `float`, `bool`, and `str`.

## Integers

`int` is a 64-bit signed integer. Integer literals can be written in decimal or hexadecimal, and `_` may be used anywhere as a visual separator (it is ignored).

```mimas
let a: int = 0;
let b: int = 0xff;          // 255, in hex
let c: int = 1_000_000;     // underscores are just for readability
```

Integer arithmetic is **checked**: an operation that overflows the 64-bit range is a runtime error rather than silently wrapping around.

```mimas
let big = 9223372036854775807; // i64::MAX
let oops = big + 1;            // runtime error: integer arithmetic overflowed
```

## Floats

`float` is a 64-bit IEEE-754 floating-point number.

```mimas
let a: float = 0.0;
let b: float = 1_000.000_1;
```

An `int` and a `float` can be combined in one expression, but the result is always a `float` -- the `int` is promoted. There is no implicit `float` -> `int`.

```mimas
let a: float = 1 + 0.1; // valid -- (int + float) => float
let b: int = 1 + 0.1;   // compile error: expected int, found float
```

````admonish warning title="Division always yields a float"
The `/` operator *always* produces a `float`, even between two `int`s. When you want integer (truncating) division, use the `~/` operator.

```mimas
let a: float = 7 / 2;  // 3.5
let b: int   = 7 ~/ 2; // 3
```
````

To convert deliberately between the two, use the conversion methods rather than a cast -- mimas has no `as`.

```mimas
let n: int = 42;
let f: float = n.to_float(); // 42.0
let back: int = f.to_int();  // 42 (truncates toward zero)
```

## Booleans

`bool` is `true` or `false`. Comparisons and logical operators produce `bool`s, and conditions in `if`, `while`, and friends must be `bool`.

```mimas
let a: bool = true;
let b = 3 > 2;        // true
let c = a && !b;      // false
```

See [Operators](./operations.md) for the full set of comparison and logical operators.

## Strings

Text is always `str`. mimas has no separate character type -- a single character is just a one-character `str`. Strings use double quotes only (`"`), never single (`'`).

```mimas
let a: str = "hello!";
let b: str = "x"; // a one-character str, not a `char`
```

Any string literal may span multiple lines; the newline becomes part of the value.

```mimas
let poem = "this string has
multiple lines!"; // -> "this string has\nmultiple lines!"
```

### Interpolation (f-strings)

Prefix a literal with `f` to interpolate expressions inside `{ }`. Any expression is allowed.

```mimas
let name = "world";
let greeting = f"hello, {name}! 1 + 1 = {1 + 1}"; // -> "hello, world! 1 + 1 = 2"
```

To write a literal brace, double it:

```mimas
let s = f"{{not interpolated}} but {1 + 1} is"; // -> "{not interpolated} but 2 is"
```

````admonish tip title="String methods chain"
`str` has a wide set of built-in methods, and they chain:

```mimas
let cleaned = "  Hello, World  ".trim().to_lower(); // -> "hello, world"
```
````
