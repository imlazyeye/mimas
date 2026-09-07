# Enums

An enum is a named sum type: a value is exactly **one** of several variants. Each variant has its own shape and can carry data, either as a tuple of types, as named fields, or as nothing at all.

```mimas
enum Shape {
    Circle(float),           // tuple variant
    Rectangle(float, float), // tuple variant
    Labeled { text: str },   // struct variant
    Empty,                   // payload-free variant
}

let a = Shape::Circle(1.0);
let b = Shape::Rectangle(2.0, 3.0);
let c = Shape::Labeled { text = "square" };
let d = Shape::Empty;
```

Variants are namespaced under the enum with `::`. A payload-free variant like `Shape::Empty` is used directly as a value; the others are constructed by supplying their data -- positionally for tuple variants, with `=` for struct variants (mirroring [struct](./structs.md) construction).

```admonish note title="An enum is always one of its variants"
You can't make a bare `Shape()`. There's no such thing as an enum value that isn't one specific variant. Construct through a variant, always.
```

## Matching on variants

[`match`](../control-flow/match.md) is how you take an enum apart: each arm names a variant and binds its payload. Because the compiler knows the full variant list, it can check that you've covered them all.

```mimas
fn area(s: Shape) -> float {
    match s {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rectangle(w, h) => w * h,
        Shape::Labeled { text } => 0.0,
        Shape::Empty => 0.0,
    }
}
```

## Methods

Like structs, enums take `impl` blocks for associated items and methods. A method that dispatches on `self` is the idiomatic way to fold behavior into the type itself:

```mimas
impl Shape {
    fn area(self) -> float {
        match self {
            Shape::Circle(r) => 3.14159 * r * r,
            Shape::Rectangle(w, h) => w * h,
            _ => 0.0,
        }
    }
}

let total = Shape::Circle(2.0).area(); // 12.56636
```
