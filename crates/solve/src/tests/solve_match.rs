use crate::components::Ty::*;

test_ty!(
    match_default,
    "match 0 {
        0 => 0,
        _ => 1
    }" => Int,
);
test_ty!(
    match_multi_case,
    "match 1 {
        0 | 1 => true,
        _ => false
    }" => Bool
);
test_ty!(
    match_binding,
    "let result = match 42 { 0 => 0, n => n + 1 };",
    "result" => Int
);
test_ty!(
    match_arms_coerce_option,
    "let result = match 0 { 0 => 1, _ => null };",
    "result" => option!(Int)
);
test_ty!(
    match_guard,
    "match 5 {
        5 if true => 1,
        _ => 2
    }" => Int
);
test_ty!(
    match_panic_terminator,
    "match 0 {
        0 => 1,
        1 => 2,
        !
    }" => Int,
);

// arm body type mismatch
test_fail!(
    match_arms_type_mismatch,
    "let x = match 0 { 0 => 1, _ => \"x\" };"
);

// InvalidPattern: literal pattern against an incompatible scrutinee
test_fail!(
    pattern_against_incompatible_scrutinee,
    "let x: int = 5; let _ = match x { \"a\" => 0, _ => 1 };",
    "let x = true; let _ = match x { 0 => 0, _ => 1 };",
    "let x = \"a\"; let _ = match x { true => 0, _ => 1 };",
);

// bool
test_ty!(
    match_bool_exhaustive,
    "let b = true;",
    "match b { true => 1, false => 2 }" => Int,
);
test_ty!(
    match_bool_or_pattern_exhaustive,
    "match true { true | false => 1 }" => Int,
);
test_fail!(
    match_bool_non_exhaustive,
    "let x = match true { true => 1 };",
    "let x = match true { false => 1 };",
);

// guards never exhaust
test_fail!(
    match_guards_dont_exhaust,
    "let b = true;
     let x = match b { true if false => 1, false => 2 };",
    "let x = match 5 { 5 if true => 1 };",
);

// literal exhaustiveness
test_fail!(match_literal_non_exhaustive, "let x = match 0 { 0 => 0 };");
test_fail!(
    match_multiple_literal_arms_non_exhaustive,
    "let x = match 0 {
        0 => true,
        1 => false
    };"
);

// or-patterns
test_ty!(
    match_or_pattern_with_wildcard,
    "match 5 { 0 | 1 => 1, _ => 2 }" => Int,
);
test_fail!(
    match_or_pattern_partial_non_exhaustive,
    "let x = match 5 { 0 | 1 => 1 };"
);
test_fail!(
    or_bindings_one_side_only,
    "let x = 5; let _ = match x { n | 1 => 0, _ => 1 };"
);

// tuples
test_ty!(
    match_tuple_wildcard_exhaustive,
    "let t = (1, 2);",
    "match t { (a, b) => a + b }" => Int,
);
test_ty!(
    match_tuple_full_wildcard_exhaustive,
    "let t = (1, 2);",
    "match t { _ => 0 }" => Int,
);
test_ty!(
    match_tuple_bool_combinatorial,
    "let t = (true, false);",
    "match t {
        (true, true) => 1,
        (true, false) => 2,
        (false, true) => 3,
        (false, false) => 4
    }" => Int,
);
test_fail!(
    match_tuple_missing_combination_non_exhaustive,
    "let t = (true, false);
     let x = match t {
         (true, true) => 1,
         (false, _) => 2
     };"
);
test_fail!(
    match_tuple_l_shape_non_exhaustive,
    "let t = (true, false);
     let x = match t {
         (true, _) => 1,
         (_, true) => 2
     };"
);

// structs
test_ty!(
    match_struct_binds_fields,
    "struct Pair { a: int, b: int }
     let p = Pair { a = 1, b = 2 };",
    "match p { Pair { a, b } => a + b }" => Int,
);
test_ty!(
    match_struct_with_literal_field,
    "struct Pair { a: int, b: int }
     let p = Pair { a = 1, b = 2 };",
    "match p {
        Pair { a = 0, b } => b,
        Pair { a, b } => a + b
    }" => Int,
);
test_ty!(
    match_struct_subset_of_fields,
    "struct Pair { a: int, b: int }
     let p = Pair { a = 1, b = 2 };",
    "match p { Pair { a } => a }" => Int,
);
test_fail!(
    match_struct_unknown_field,
    "struct Pair { a: int, b: int }
     let p = Pair { a = 1, b = 2 };
     let x = match p { Pair { c } => c };"
);
test_fail!(
    match_struct_literal_field_non_exhaustive,
    "struct Pair { a: int, b: int }
     let p = Pair { a = 1, b = 2 };
     let x = match p { Pair { a = 0, b } => b };"
);

// tuple structs
test_ty!(
    match_tuple_struct,
    "struct Wrap(int);
     let w = Wrap(7);",
    "match w { Wrap(x) => x }" => Int,
);
test_ty!(
    match_tuple_struct_literal,
    "struct Wrap(int);
     let w = Wrap(7);",
    "match w {
        Wrap(0) => 1,
        Wrap(n) => n
    }" => Int,
);
test_fail!(
    match_tuple_struct_wrong_arity,
    "struct Wrap(int);
     let w = Wrap(7);
     let x = match w { Wrap(a, b) => a };"
);

// enums
test_ty!(
    match_enum_tuple_variant_binds,
    "enum Shape { Circle(int), Square(int, int) }
     let s = Shape::Circle(3);",
    "match s {
        Shape::Circle(r) => r,
        Shape::Square(a, b) => a + b
    }" => Int,
);
test_ty!(
    match_enum_struct_variant_binds,
    "enum Msg { Quit, Move { x: int, y: int } }
     let m = Msg::Move { x = 1, y = 2 };",
    "match m {
        Msg::Quit => 0,
        Msg::Move { x, y } => x + y
    }" => Int,
);
test_ty!(
    match_enum_mixed_variants_exhaustive,
    "enum Color { Red, Rgb(int, int, int), Named { name: str } }
     let c = Color::Red {};",
    "match c {
        Color::Red => 0,
        Color::Rgb(_r, _g, _b) => 1,
        Color::Named { name = _n } => 2
    }" => Int,
);
test_ty!(
    match_enum_all_variants_exhaustive,
    "enum Color { Red, Blue }
     let c = Color::Red {};",
    "match c { Color::Red => 0, Color::Blue => 1 }" => Int,
);
test_ty!(
    match_enum_wildcard_covers_missing,
    "enum Color { Red, Blue, Green }
     let c = Color::Red {};",
    "match c { Color::Red => 0, _ => 1 }" => Int,
);
test_ty!(
    match_enum_tuple_variant_with_wildcards_exhaustive,
    "enum Shape { Circle(int), Square(int, int) }
     let s = Shape::Circle(3);",
    "match s {
        Shape::Circle(_) => 0,
        Shape::Square(_, _) => 1
    }" => Int,
);
test_ty!(
    match_enum_struct_variant_partial_field_set_exhaustive,
    "enum Msg { Move { x: int, y: int }, Stop }
     let m = Msg::Stop {};",
    "match m { Msg::Move { x } => x, Msg::Stop => 0 }" => Int,
);
test_ty!(
    match_or_enum_variants_exhaustive,
    "enum Color { Red, Blue, Green }
     let c = Color::Red {};",
    "match c { Color::Red | Color::Blue => 1, Color::Green => 2 }" => Int,
);
test_ty!(
    match_or_enum_variants,
    "enum Tri { A, B, C }
     let t = Tri::A {};",
    "match t {
        Tri::A | Tri::B => 1,
        Tri::C => 2
    }" => Int,
);
test_fail!(
    match_enum_one_variant_missing_non_exhaustive,
    "enum Color { Red, Blue, Green }
     let c = Color::Red {};
     let x = match c {
         Color::Red => 0,
         Color::Blue => 1
     };"
);
test_fail!(
    match_enum_unknown_variant,
    "enum Color { Red, Blue }
     let c = Color::Red {};
     let x = match c { Color::Green => 0, _ => 1 };"
);
test_fail!(
    match_tuple_variant_on_struct_variant,
    "enum Msg { Move { x: int, y: int } }
     let m = Msg::Move { x = 1, y = 2 };
     let x = match m { Msg::Move(a, b) => a };"
);

// Self in patterns
test_ty!(
    match_self_enum_variant,
    "enum State { On, Off }
     impl State {
         fn is_on(self) -> bool {
             match self {
                 Self::On => true,
                 Self::Off => false
             }
         }
     }",
);
test_ty!(
    match_self_struct,
    "struct Point { x: int, y: int }
     impl Point {
         fn sum(self) -> int {
             match self { Self { x, y } => x + y }
         }
     }",
);
test_ty!(
    match_self_tuple_struct,
    "struct Wrap(int);
     impl Wrap {
         fn unwrap(self) -> int {
             match self { Self(n) => n }
         }
     }",
);

// null-bind patterns
test_ty!(
    match_null_bind,
    "let a: int? = 0;",
    "match a {
        x? => x + 1,
        n => 0
    }" => Int,
);
test_ty!(
    match_null_bind_str,
    "let a: str? = \"x\";",
    "match a { \"x\"? => \"hit\", _ => \"miss\" }" => Str,
);
test_ty!(
    match_null_bind_float,
    "let a: float? = 1.0;",
    "match a { 1.0? => 1, _ => 0 }" => Int,
);
test_ty!(
    match_null_bind_hex,
    "let a: int? = 255;",
    "match a { 0xff? => 1, _ => 0 }" => Int,
);
test_ty!(
    match_null_bind_struct,
    "struct P { x: int } let p: P? = P { x = 5 };",
    "match p { P { x }? => x, _ => 0 }" => Int,
);
test_fail!(
    match_null_bind_on_non_option,
    "let a = 0;
     let x = match a { x? => x + 1, _ => 0 };"
);
