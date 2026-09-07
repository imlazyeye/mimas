# Structs

A struct is a named product type -- it bundles several fields, each with its own type, under one name.

```mimas
struct Player {
    name: str,
    score: int,
}

let p = Player {
    name = "ada",
    score = 0,
};
```

Construction uses `=` for each field (not `:`, which is reserved for type annotations). Read a field back with dot access:

```mimas
print(p.name);  // "ada"
print(p.score); // 0
```

## Methods and associated items

`impl` blocks attach behavior to a struct. A type can have any number of `impl` blocks, as long as the struct is in scope.

An `impl` can define **associated constants**, **associated functions**, and **methods**. A function becomes a method when its first parameter is `self`; `Self` refers to the struct being implemented.

```mimas
struct Player {
    name: str,
    score: int,
}

impl Player {
    const MAX_SCORE = 100;

    // associated function -- no `self`
    fn new(name: str) -> Self {
        Self { name = name, score = 0 }
    }

    // method -- takes `self`
    fn is_winner(self) -> bool {
        self.score >= Self::MAX_SCORE
    }
}

let p = Player::new("ada");   // call an associated function with `::`
print(p.is_winner());         // call a method with `.`
```

Calling a method with dot syntax is sugar that passes the receiver in as `self`. These two lines are equivalent:

```mimas
let won = p.is_winner();
let won = Player::is_winner(p);
```

## Reaching items through a value

mimas has one deliberate split from Rust: dot access can *also* reach an associated item through a value, not just through the type name. This matters for [pacts](../pacts.md), where you have a value but not its concrete type name.


```mimas
let p = Player::new("ada");
print(p.MAX_SCORE);      // 100 -- the associated const, reached through the value
print(Player::MAX_SCORE); // the same const, through the type
```

## Tuple structs

A struct can use positional fields instead of named ones, making it a **tuple struct**. Read its fields by index, like a [tuple](../collections/tuples.md).

```mimas
struct Vec2(float, float)

let v = Vec2(1.0, 2.0);
let x = v.0; // 1.0
let y = v.1; // 2.0
```

A tuple struct's bare name doubles as a **constructor value**: a function `(fields...) -> Struct` you can pass around -- while the name still works as a type in annotations.

```mimas
struct Wrap(int)

let make = Wrap;       // `make` is a function value, (int) -> Wrap
let w: Wrap = make(7); // `Wrap` is still usable as a type
print(w.0);            // 7
```

A field-free struct is written with empty braces or a bare `struct Name;` -- useful as a marker type or a home for associated items and pact impls.

```mimas
struct Marker;
```
