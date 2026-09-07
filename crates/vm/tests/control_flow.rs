#[macro_use]
mod vm_test_utils;

use vm::Captured::*;

test_vm!(
    r#if,
    "if false { 1 } else if true { 2 } else { 3 }" => Int(2),
);

test_vm!(
    if_let,
    "if let a = 0 { a } else { 1 }" => Int(0),
);

test_vm!(
    if_let_op,
    "let a: int? = 0;",
    "if let a? = a { a } else { 1 }" => Int(0),
);

test_vm!(
    if_let_op_result,
    "fn ok() -> int! { 42 }
     fn bad() -> int! { raise \"x\" }",
    "if let v? = ok() { v } else { 1 }" => Int(42),
    "if let v? = bad() { v } else { 1 }" => Int(1),
);

test_vm!(
    if_let_tuple,
    "let t = (1, 2);",
    "if let (a, b) = t { a + b } else { 0 }" => Int(3),
);

test_vm!(
    if_let_literal_int,
    "let n = 5;",
    "if let 5 = n { 1 } else { 0 }" => Int(1),
    "if let 9 = n { 1 } else { 0 }" => Int(0),
);

test_vm!(
    if_let_literal_bool,
    "let b = true;",
    "if let true = b { 1 } else { 0 }" => Int(1),
    "if let false = b { 1 } else { 0 }" => Int(0),
);

test_vm!(
    if_let_literal_str,
    r#"let s = "hi";"#,
    r#"if let "hi" = s { 1 } else { 0 }"# => Int(1),
    r#"if let "bye" = s { 1 } else { 0 }"# => Int(0),
);

test_vm!(
    if_let_or,
    "let n = 1;",
    "if let 0 | 1 = n { 1 } else { 0 }" => Int(1),
    "if let 2 | 3 = n { 1 } else { 0 }" => Int(0),
);

test_vm!(
    if_let_struct,
    "struct Pair { a: int, b: int }
     let p = Pair { a = 1, b = 2 };",
    "if let Pair { a, b } = p { a + b } else { 0 }" => Int(3),
);

test_vm!(
    if_let_tuple_struct,
    "struct Wrap(int);
     let w = Wrap(7);",
    "if let Wrap(x) = w { x } else { 0 }" => Int(7),
);

test_vm!(
    if_let_enum_unit_variant,
    "enum Color { Red, Blue }
     let c = Color::Red {};",
    "if let Color::Red = c { 1 } else { 0 }" => Int(1),
);

test_vm!(
    if_let_enum_unit_variant_no_match,
    "enum Color { Red, Blue }
     let c = Color::Blue {};",
    "if let Color::Red = c { 1 } else { 0 }" => Int(0),
);

test_vm!(
    if_let_enum_tuple_variant,
    "enum Shape { Circle(int), Square(int, int) }
     let s = Shape::Circle(3);",
    "if let Shape::Circle(r) = s { r } else { 0 }" => Int(3),
);

test_vm!(
    if_let_enum_struct_variant,
    "enum Msg { Quit, Move { x: int, y: int } }
     let m = Msg::Move { x = 1, y = 2 };",
    "if let Msg::Move { x, y } = m { x + y } else { 0 }" => Int(3),
);

test_vm!(
    if_let_option_some,
    "let x: int? = 7;",
    "if let v? = x { v + 1 } else { -1 }" => Int(8),
);

test_vm!(
    if_let_option_null,
    "let x: int? = null;",
    "if let v? = x { v } else { -1 }" => Int(-1),
);

test_vm!(
    if_let_option_zero,
    "let x: int? = 0;",
    "if let v = x { v } else { -1 }" => Int(0),
);

// let else -- hosted in fns so the divergent `else` (return/break/continue) has somewhere to go.
// the bound name flows into the code *after* the `let`; the `else` runs when the pattern fails.
test_vm!(
    let_else_option,
    "fn run(x: int?) -> int { let v? = x else return -1; v + 1 }",
    "run(7)" => Int(8),
    "run(null)" => Int(-1),
);

test_vm!(
    let_else_result,
    "fn ok() -> int! { 42 }
     fn bad() -> int! { raise \"x\" }
     fn use_ok() -> int { let v? = ok() else return -1; v }
     fn use_bad() -> int { let v? = bad() else return -1; v }",
    "use_ok()" => Int(42),
    "use_bad()" => Int(-1),
);

test_vm!(
    let_else_enum_tuple_variant,
    "enum Shape { Circle(int), Square(int, int) }
     fn radius(s: Shape) -> int { let Shape::Circle(r) = s else return -1; r }",
    "radius(Shape::Circle(3))" => Int(3),
    "radius(Shape::Square(2, 4))" => Int(-1),
);

test_vm!(
    let_else_enum_struct_variant,
    "enum Msg { Quit, Move { x: int, y: int } }
     fn delta(m: Msg) -> int { let Msg::Move { x, y } = m else return -1; x + y }",
    "delta(Msg::Move { x = 1, y = 2 })" => Int(3),
    "delta(Msg::Quit {})" => Int(-1),
);

test_vm!(
    let_else_literal,
    "fn is_five(n: int) -> int { let 5 = n else return 0; 1 }",
    "is_five(5)" => Int(1),
    "is_five(9)" => Int(0),
);

test_vm!(
    let_else_or,
    "fn small(n: int) -> int { let 0 | 1 = n else return -1; 1 }",
    "small(0)" => Int(1),
    "small(1)" => Int(1),
    "small(2)" => Int(-1),
);

test_vm!(
    let_else_nested_option_in_variant,
    "enum Slot { Full(int?), Empty }
     fn inner(s: Slot) -> int { let Slot::Full(v?) = s else return -1; v }",
    "inner(Slot::Full(7))" => Int(7),
    "inner(Slot::Full(null))" => Int(-1),
    "inner(Slot::Empty {})" => Int(-1),
);

test_vm!(
    let_else_continue,
    "fn sum_present(xs: [int?]) -> int {
         let total = 0;
         for x in xs {
             let v? = x else continue;
             total += v;
         }
         total
     }",
    "sum_present([1, null, 2, null, 3])" => Int(6),
);

test_vm!(
    let_else_break,
    "fn prefix_sum(xs: [int?]) -> int {
         let total = 0;
         for x in xs {
             let v? = x else break;
             total += v;
         }
         total
     }",
    "prefix_sum([1, 2, null, 3])" => Int(3),
    "prefix_sum([null, 9])" => Int(0),
);

test_vm!(
    r#loop,
    "loop break 0" => Int(0),
    "loop break 42" => Int(42),
    r#"loop break "hi""# => str!("hi"),
);

test_vm!(
    loop_break_value_bound_typed,
    "let r: int = loop break 5;",
    "r + 1" => Int(6),
);

test_vm!(
    r#while,
    "while true break 0" => Int(0),
    "while false break 0" => Null,
);

test_vm!(
    while_let,
    "while let foo = 0 break foo" => Int(0),
);

test_vm!(
    // drains an option source: binds the unwrapped value each pass, stops the first null.
    // also a guard against the infinite-loop regression (no null short-circuit).
    while_let_drains,
    "let n = 0;
     fn next(i: int) -> int? { if i < 3 { i } else { null } }
     let sum = 0;
     while let v = next(n) { sum += v; n += 1; }",
    "sum" => Int(3),
);

test_vm!(
    while_let_null_stops,
    "let x: int? = null;
     let ran = 0;
     while let v = x { ran += 1; }",
    "ran" => Int(0),
);

test_vm!(
    while_collect,
    "let x = 0;",
    "while x == 0 {
        x = -1;
        collect 0;
    }" => array!(Int(0))
);

test_vm!(
    r#for,
    "for x in [0] {}" => Null,
);

test_vm!(
    for_in_str,
    r#"for x in ["x", "y", "z"] { collect x; }"# => array!(str!("x"), str!("y"), str!("z"))
);

test_vm!(
    for_in_int,
    r#"for x in 3 { collect x; }"# => array!(Int(0), Int(1), Int(2))
);

test_vm!(
    for_break,
    "for foo in [0, 1, 2] break foo" => Int(0),
    "for foo in [0, 1, 2] {
        if foo == 2 {
            break foo;
        }
        break foo;
    }" => Int(0),
);

test_vm!(
    for_collect,
    "let foo = [0, 1, 2];",
    "for x in foo collect x" => array!(Int(0), Int(1), Int(2)),
);

test_vm!(
    for_over_str,
    r#"for ch in "abc" collect ch"# => array!(str!("a"), str!("b"), str!("c")),
);

test_vm!(
    while_with_counter,
    "let i = 0;
     while i < 5 {
        i += 1;
     };",
    "i" => Int(5),
);

test_vm!(
    continue_in_for,
    "let count = 0;
     for x in [1, 2, 3, 4, 5] {
        if x % 2 == 0 { continue; }
        count += 1;
     }",
    "count" => Int(3),
);

test_vm!(
    for_collect_filter,
    "for x in [1, 2, 3, 4, 5] {
        if x > 2 {
            collect x;
        }
    }" => array!(Int(3), Int(4), Int(5)),
);

test_vm!(
    for_collect_transform,
    "let a = [1, 2, 3];",
    "for x in a collect x * 2" => array!(Int(2), Int(4), Int(6)),
);

test_vm!(
    for_collect_empty,
    "for x in [] collect x" => array!(),
);

test_vm!(
    for_collect_bound_to_let,
    "let r = for x in [1, 2, 3] collect x + 10;",
    "r" => array!(Int(11), Int(12), Int(13)),
);

test_vm!(
    for_collect_string_chars,
    "let s = \"abc\";",
    "for c in s collect c" => array!(str!("a"), str!("b"), str!("c")),
);

test_vm!(
    for_array_iter_hoisted_len,
    "let a = [10, 20, 30]; let total = 0; for x in a { total = total + x; }",
    "total" => Int(60),
);

test_vm!(
    if_let_on_self_field,
    "struct Foo { val: int? }
     impl Foo {
         fn check(self) -> int {
             if let v? = self.val {
                 return v;
             }
             return 0;
         }
     }
     let f = Foo { val = 5 };",
    "f.check()" => Int(5),
);
