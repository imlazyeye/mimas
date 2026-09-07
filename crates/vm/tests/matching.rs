#[macro_use]
mod vm_test_utils;

use vm::Captured::*;

test_vm!(
    match_literal,
    "let x = 1;",
    "match x { 0 => 0, 1 => 1, _ => 2 }" => Int(1),
);

test_vm!(
    match_ident_binding,
    "match 5 { n => n + 1 }" => Int(6),
);

test_vm!(
    match_panic_terminator,
    "let b = true;",
    "match b { true => 1, false => 2, ! }" => Int(1)
);

test_vm!(
    match_guard,
    "match 5 { n if n > 3 => 0, _ => 1 }" => Int(0)
);

test_vm!(
    match_or_pattern,
    "let n = 2;",
    "match n { 0 | 1 | 2 => true, _ => false }" => Bool(true),
);

test_vm!(
    match_or_binding,
    "enum Foo { A(int), B(int) }",
    "match Foo::A(3) { Foo::A(n) | Foo::B(n) => n }" => Int(3),
    "match Foo::B(9) { Foo::A(n) | Foo::B(n) => n }" => Int(9),
);

test_vm!(
    match_tuple,
    "let t = (1, 2);",
    "match t { (a, b) => a + b }" => Int(3),
);

// jump-table dispatch: exhaustive variant match (no wildcard -> panic default).
test_vm!(
    match_switch_exhaustive,
    "enum Shape { Circle(int), Square(int, int), Tri(int, int) }",
    "match Shape::Square(3, 4) { Shape::Circle(r) => r, Shape::Square(w, h) => w * h, Shape::Tri(b, h) => b + h }" => Int(12),
);

// jump-table dispatch: a variant not named explicitly lands on the wildcard default.
test_vm!(
    match_switch_wildcard_default,
    "enum E { A(int), B(int), C(int) }",
    "match E::C(9) { E::A(x) => x, E::B(x) => x + 1, _ => 99 }" => Int(99),
);

// jump-table dispatch binds struct-variant fields after the tag jump.
test_vm!(
    match_switch_struct_variant,
    "enum Msg { Quit, Move { x: int, y: int } }
     let m = Msg::Move { x = 5, y = 6 };",
    "match m { Msg::Quit => 0, Msg::Move { x, y } => x + y }" => Int(11),
);

// jump-table dispatch: a binding catch-all is the default, and binds the whole scrutinee.
test_vm!(
    match_switch_binding_default,
    "enum E { A(int), B(int), C(int) }",
    "match E::B(4) { E::A(x) => x, other => 7 }" => Int(7),
);

// jump-table dispatch with a fieldless or-pattern: two variant tags route to the one arm body.
test_vm!(
    match_switch_or_fieldless,
    "enum Color { Red, Green, Blue, Yellow }",
    "match Color::Green { Color::Red | Color::Green => 1, Color::Blue => 2, Color::Yellow => 3 }" => Int(1),
    "match Color::Yellow { Color::Red | Color::Green => 1, Color::Blue => 2, Color::Yellow => 3 }" => Int(3),
);

// jump-table dispatch with a payload-binding or-pattern: each tag reaches the shared body, which
// binds the (slot-uniform) payload once -- correct whichever variant matched.
test_vm!(
    match_switch_or_binding,
    "enum E { A(int), B(int), C(int) }",
    "match E::A(5) { E::A(n) | E::B(n) => n * 2, E::C(n) => n }" => Int(10),
    "match E::B(6) { E::A(n) | E::B(n) => n * 2, E::C(n) => n }" => Int(12),
);

// an unmatched variant whose tag sits inside the table's range still lands on the wildcard default.
test_vm!(
    match_switch_inrange_default,
    "enum E { A(int), B(int), C(int), D(int) }",
    "match E::B(0) { E::A(_) => 1, E::C(_) => 3, E::D(_) => 4, _ => 9 }" => Int(9),
);

// or-pattern whose alternatives bind the same names at DIFFERENT positions can't share one body
// bind, so it lowers via the is_instance chain -- each alternative must still bind from its own
// slots. `Swap(b, a)` swaps the payloads relative to `Pair(a, b)`.
test_vm!(
    match_or_reordered_fields,
    "enum E { Pair(int, int), Swap(int, int) }",
    "match E::Pair(10, 3) { E::Pair(a, b) | E::Swap(b, a) => a - b }" => Int(7),
    "match E::Swap(10, 3) { E::Pair(a, b) | E::Swap(b, a) => a - b }" => Int(-7),
);

// first-match order: a catch-all before a variant arm wins, even though the switch would otherwise
// route that variant elsewhere. regression for the wildcard-not-last ordering bug.
test_vm!(
    match_wildcard_before_variant,
    "enum E { A(int), B(int) }",
    "match E::A(1) { _ => 100, E::A(x) => x }" => Int(100),
);

test_vm!(
    match_tuple_literal_position,
    "let t = (1, 5);",
    "match t { (1, n) => n, (_, n) => n * 10 }" => Int(5),
);

test_vm!(
    match_enum_bare_variant,
    "enum Color { Red, Blue }
     let c = Color::Red {};",
    "match c { Color::Red => 1, Color::Blue => 2 }" => Int(1),
);

test_vm!(
    match_enum_tuple_variant_binds,
    "enum Shape { Circle(int), Square(int, int) }
     let s = Shape::Square(3, 4);",
    "match s { Shape::Circle(r) => r, Shape::Square(a, b) => a + b }" => Int(7),
);

test_vm!(
    match_enum_struct_variant_binds,
    "enum Msg { Quit, Move { x: int, y: int } }
     let m = Msg::Move { x = 7, y = 11 };",
    "match m { Msg::Quit => 0, Msg::Move { x, y } => x + y }" => Int(18),
);

test_vm!(
    match_struct_destructure,
    "struct Pair { a: int, b: int }
     let p = Pair { a = 4, b = 9 };",
    "match p { Pair { a, b } => a + b }" => Int(13),
);

test_vm!(
    match_tuple_struct_destructure,
    "struct Wrap(int);
     let w = Wrap(42);",
    "match w { Wrap(n) => n }" => Int(42),
);

test_vm!(
    match_null_bind_some,
    "let a: int? = 7;",
    "match a { x? => x + 1, _ => 0 }" => Int(8),
);

test_vm!(
    match_null_bind_none,
    "let a: int? = null;",
    "match a { x? => x + 1, _ => 0 }" => Int(0),
);

test_vm!(
    match_null_bind_literal_hit,
    "let a: str? = \"foo\";",
    "match a { \"foo\"? => 1, _ => 0 }" => Int(1),
);

test_vm!(
    match_null_bind_literal_miss,
    "let a: str? = \"bar\";",
    "match a { \"foo\"? => 1, _ => 0 }" => Int(0),
);

test_vm!(
    match_null_bind_literal_null,
    "let a: str? = null;",
    "match a { \"foo\"? => 1, _ => 0 }" => Int(0),
);

test_vm!(
    match_null_bind_struct,
    "struct P { x: int } let p: P? = P { x = 5 };",
    "match p { P { x }? => x, _ => -1 }" => Int(5),
);

test_vm!(
    match_null_bind_true,
    "let a: bool? = true;",
    "match a { true? => 1, _ => 0 }" => Int(1),
);

test_vm!(
    match_null_bind_float,
    "let a: float? = 1.5;",
    "match a { 1.5? => 1, _ => 0 }" => Int(1),
);

test_vm!(
    match_self_in_method,
    "enum State { On, Off }
     impl State {
         fn flag(self) -> int {
             match self { Self::On => 1, Self::Off => 0 }
         }
     }
     let s = State::On {};",
    "s.flag()" => Int(1),
);

// `Self::Variant {}` in an impl method body -- `Ty::Identity` left-side of a DoubleColon path.
// previously `Literal::Struct` only matched `Ty::Adt` and rejected this with "Self is not a struct"
test_vm!(
    self_variant_constructor_in_method,
    "enum Light { On, Off }
     impl Light {
         fn flip(self) -> Self {
             match self {
                 Self::On => Self::Off {},
                 Self::Off => Self::On {},
             }
         }
         fn is_on(self) -> bool {
             match self { Self::On => true, Self::Off => false }
         }
     }
     let l = Light::On {}.flip();",
    "l.is_on()" => Bool(false),
);
