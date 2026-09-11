use crate::components::Ty::*;

// Sets and access
test_ty!(array, "[0]" => array!(Int));
test_ty!(tuple, "(0,)" => tuple!(Int));
test_ty!(triple_tuple, "(0, 0, 0)" => tuple!(Int, Int, Int));
test_ty!(dictionary, "~{ a = 0 }" => dictionary!(Int));
test_ty!(coerced_array, "[0, null]" => array!(option!(Int)));
test_ty!(coerced_dictionary, "~{ a = 0, b = null }" => dictionary!(option!(Int)));
test_ty!(coerced_tuple_with_null, "(0, null)" => tuple!(Int, Null));
test_ty!(
    array_access,
    "let a = [0];",
    "a[0]" => Int,
);
test_ty!(
    tuple_access,
    "let a = (0, 1);",
    "a.0" => Int,
);
test_fail!(
    tuple_access_out_of_bounds,
    "let a = (0, 1);
    let b = a.2;"
);
test_ty!(
    optional_array_access,
    "let a: [int]? = null;",
    "a?[0]" => option!(Int),
);
test_ty!(
    dictionary_access,
    "let a = ~{ foo = [0] };",
    "a[\"foo\"]" => option!(array!(Int)),
);
test_ty!(
    optional_dictionary_access,
    "let a: ~{int}? = null;",
    "a?[\"foo\"]" => option!(Int),
);
test_ty!(
    chained_sets,
    "let a = [ ~{ foo = [ [0] ] } ];",
    "a[0][\"foo\"]?[0][0]" => option!(Int),
);
test_ty!(
    chained_optional_set_accesses,
    "let a: [[~{int}?]?]? = null;",
    "a?[0]?[0]?[\"foo\"]" => option!(Int),
);
test_ty!(dict_index_is_option, "let d = ~{ a = 0 };", "d[\"a\"]" => option!(Int));
test_fail!(
    plain_index_after_dict,
    "let d = ~{ a = ~{ b = 0 } }; let y = d[\"a\"][\"b\"];"
);
test_fail!(
    plain_dot_on_option,
    "struct Foo { x: int }
     let a: Foo? = null;
     let y = a.x;"
);
test_ty!(
    taint_square_then_dot,
    "struct Foo { x: int }
     let a: [Foo]? = null;",
    "a?[0].x" => option!(Int),
);
test_ty!(
    taint_dot_then_square,
    "struct Foo { items: [int] }
     let a: Foo? = null;",
    "a?.items[0]" => option!(Int),
);
test_fail!(
    taint_stops_at_grouping,
    "let a = [ ~{ foo = [[0]] } ]; let y = (a[0][\"foo\"]?[0])[0];"
);
// one `?` per dict you reach through. dict<dict<array>>: the `["y"]` miss needs its own `?`, so a
// single `?` then plain `[0]` is rejected; `?[0]` is required.
test_fail!(
    dict_dict_array_one_question,
    "let f: ~{~{[int]}} = ~{}; let y = f[\"x\"]?[\"y\"][0];"
);
test_ty!(
    dict_dict_array_two_questions,
    "let f: ~{~{[int]}} = ~{};",
    "f[\"x\"]?[\"y\"]?[0]" => option!(Int),
);
// a terminal dict option is fine -- you only need a `?` to reach *through* one.
test_ty!(
    dict_dict_terminal_option,
    "let f: ~{~{int}} = ~{};",
    "f[\"x\"]?[\"y\"]" => option!(Int),
);

// Blocks
test_ty!(block_no_yield, "{}" => Unit);
test_ty!(block_yield, "{ 0 }" => Int);
test_fail!(
    block_isolated_scope,
    "{ let x = 0; }
    let y = x;"
);

// Loops / control flows
test_ty!(loop_infinite, "loop {}" => Never);
test_ty!(loop_unit, "loop { break; }" => Unit);
test_ty!(loop_value, "loop { break 0; }" => Int);
test_ty!(loop_collect, "loop { collect 0; break; }" => array!(Int));
test_ty!(loop_collect_infinite, "loop { collect 0; }" => Never);
test_ty!(loop_with_only_continue, "loop { continue; }" => Never);
test_ty!(while_empty, "while true {}" => Unit);
test_ty!(while_value, "while true { break 0; }" => option!(Int));
test_ty!(while_break_str, "while true { break \"a\"; }" => option!(Str));
test_ty!(while_let, "while let a = 0 { break a + 1; }" => option!(Int));
test_ty!(
    while_let_on_option,
    "let a: int? = 0;
    let c = while let b = a { break b + 1; };",
    "c" => option!(Int)
);
test_ty!(while_collect, "while true { collect 0; }" => array!(Int));
test_ty!(for_in, "for x in [] {}" => Unit);
test_ty!(for_in_str, "for x in \"foo\" { collect x }" => array!(Str));
test_ty!(for_in_int, "for x in 5 { collect x }" => array!(Int));
test_ty!(for_in_break, "for x in [] { break; }" => Unit);
test_ty!(for_in_break_unit, "for x in [] { break (); }" => Unit);
test_ty!(for_in_break_value, "for x in [] { break 0; }" => option!(Int));
test_ty!(for_in_dict, "for (x, y) in ~{ foo = 10 } { break y; }" => option!(Int));
test_ty!(
    for_in_break_option,
    "for x in [] {
        if true {
            break 0;
        } else {
            break null;
        }
    }" => option!(Int)
);
test_ty!(
    for_in_nested_break,
    "for x in [0, 1, 2] {
        break for y in [3, 4, 5] {
            break y;
        };
    }" => option!(Int)
);
test_ty!(for_in_collect, "for x in [0] { collect x; }" => array!(Int));
test_ty!(
    for_in_yields_element_type,
    "let a = [1.0, 2.0];
     let b = for x in a { collect x; };",
    "b" => array!(Float)
);
test_fail!(for_in_no_mut_iter, "for i in 1 { i = 0; }");
test_ty!(
    nested_collection,
    "let a = [0, 1, 2];
    let b = [0, 1, 2];",
    "for x in a {
        collect for y in b {
            collect y;
        };
    }" => array!(array!(Int))
);
test_ty!(
    for_in_collect_and_break,
    "for x in [0, 1, 2] {
        if x >= 1 {
            collect x;
        } else {
            break;
        }
    }" => array!(Int)
);
test_fail!(for_in_drop_value, "for x in [] { 0 }");
test_fail!(
    for_in_break_while_collection,
    "for x in [] { break 0; collect 0; }"
);
test_ty!(
    for_over_dict,
    r#"let d = ~{ a = 1, b = 2 };
    let keys = for (k, v) in d collect k;"#,
    "keys" => array!(Str)
);

test_fail!(loop_break_while_collection, "loop { break 0; collect 0; }");
test_fail!(break_out_of_context, "break;");
test_fail!(collect_out_of_context, "collect 0;");
test_fail!(continue_out_of_context, "continue;");
test_fail!(return_out_of_context, "return;");

// Equality
test_ty!(
    equalities,
    "1 == 1" => Bool,
    "1 != 1" => Bool,
    "1 > 1" => Bool,
    "1 >= 1" => Bool,
    "1 < 1" => Bool,
    "1 <= 1" => Bool,
    "1 == 1.0" => Bool,
);
test_ty!(
    option_equality,
    "let a: int? = 1;",
    "a == 1" => Bool,
    "a == null" => Bool,
);
test_fail!(error_from_invalid_type_comp, "1 == true;");
test_fail!(error_from_invalid_ord_comp, "1 > true;");
test_ty!(string_ordering, "\"a\" > \"b\"" => Bool);
test_fail!(string_int_ordering, "let a = \"a\" > 1;");
test_fail!(
    string_option_ordering,
    "let s: str? = null; let a = s < \"b\";"
);
test_fail!(bool_ordering, "let a = true > false;");
test_fail!(array_ordering, "let a = [] > [];");
test_fail!(dict_ordering, "let a = ~{} > ~{};");
test_fail!(null_ordering, "let a = null > null;");

// Evaluations
test_ty!(
    numerical_evaluations,
    "1 + 1" => Int,
    "1.0 * 1.0" => Float,
    "1.0 / 1.0" => Float,
    "1 % 1" => Int,
    "1 ~/ 1" => Int,
    "1 & 1" => Int,
    "1 | 1" => Int,
    "1 ^ 1" => Int,
    "1 << 1" => Int,
    "1 >> 1" => Int,
);
test_ty!(
    int_division_yields_float,
    "1 / 1" => Float,
);
test_ty!(
    int_float_mixed_arithmetic,
    "1 + 0.1" => Float,
    "1 - 0.1" => Float,
    "1 * 0.1" => Float,
    "0.1 + 1" => Float,
);
test_ty!(
    tuple_evaluations,
    "(0, 0) + (0, 0)" => tuple!(Int, Int),
    "(0, 0) - (0, 0)" => tuple!(Int, Int),
    "(0, 0) / (0, 0)" => tuple!(Float, Float),
    "(0, 0) * (0, 0)" => tuple!(Int, Int),
    "(0, 0) ~/ (0, 0)" => tuple!(Int, Int),
    "(0, 0) % (0, 0)" => tuple!(Int, Int),
    "(0, 0) ^ (0, 0)" => tuple!(Int, Int),
    "(0, 0) << (0, 0)" => tuple!(Int, Int),
    "(0, 0) >> (0, 0)" => tuple!(Int, Int),
    "(0, 0.0) + (0.0, 0)" => tuple!(Float, Float),
    "(true, false) & (false, true)" => tuple!(Bool, Bool),
);
test_ty!(string_addition, r#""hi" + "hi""# => Str);
test_fail!(tuple_eval_mismatch, "let a = (0, 0) + (0, 0, 0);");
test_fail!(error_from_float_bitwise, "1.0 & 1.0;");
test_fail!(bitwise_not_on_float, "let a = ~1.0;");
test_fail!(bitwise_not_on_bool, "let a = ~true;");
test_fail!(string_plus_int, "let a = \"foo\" + 1;");
test_fail!(string_subtraction, "let a = \"foo\" - \"bar\";");
test_fail!(add_option_int, "let a: int? = 1; let b = a + 1;");

// Groupings
test_ty!(empty_grouping, "()" => Unit);
test_ty!(grouping, "(0)" => Int);

// If
test_ty!(empty_if, "if true {}" => Unit);
test_ty!(empty_if_else, "if true {} else {}" => Unit);
test_ty!(filled_if_else, "if true { 0 } else { 0 }" => Int);
test_ty!(chained_if_else, "if true { 0 } else if false { 0 } else { 0 }" => Int);
test_fail!(non_bool_if_condition, "if 0 {}");
test_fail!(non_unit_if_without_else, "if true { 0 }");
test_fail!(
    if_else_mismatched_types,
    "if true { 0 } else if true { 0 } else { [] }"
);
test_fail!(
    bind_on_refutable_if_flow,
    "let a: int = if true { 0 } else if false { 1 };"
);

// if let
test_ty!(if_let, "if let a = 0 {}" => Unit);
test_ty!(
    if_let_tuple,
    "let a = (0, 1.0);
    let d = if let (b, c) = a { c } else { 0.0 };",
    "d" => Float,
);
test_ty!(
    if_let_op,
    "let a: int? = null;",
    "if let a? = a { a } else { 1 }" => Int,
);
test_ty!(
    if_let_literal_int,
    "let n = 5;",
    "if let 5 = n { 1 } else { 0 }" => Int,
);
test_ty!(
    if_let_literal_bool,
    "let b = true;",
    "if let true = b { 1 } else { 0 }" => Int,
);
test_ty!(
    if_let_literal_str,
    r#"let s = "hi";"#,
    r#"if let "hi" = s { 1 } else { 0 }"# => Int,
);
test_ty!(
    if_let_struct,
    "struct Pair { a: int, b: int }
     let p = Pair { a = 1, b = 2 };",
    "if let Pair { a, b } = p { a + b } else { 0 }" => Int,
);
test_ty!(
    if_let_tuple_struct,
    "struct Wrap(int);
     let w = Wrap(7);",
    "if let Wrap(x) = w { x } else { 0 }" => Int,
);
test_ty!(
    if_let_enum_unit_variant,
    "enum Color { Red, Blue }
     let c = Color::Red {};",
    "if let Color::Red = c { 1 } else { 0 }" => Int,
);
test_ty!(
    if_let_enum_tuple_variant,
    "enum Shape { Circle(int), Square(int, int) }
     let s = Shape::Circle(3);",
    "if let Shape::Circle(r) = s { r } else { 0 }" => Int,
);
test_ty!(
    if_let_enum_struct_variant,
    "enum Msg { Quit, Move { x: int, y: int } }
     let m = Msg::Move { x = 1, y = 2 };",
    "if let Msg::Move { x, y } = m { x + y } else { 0 }" => Int,
);
test_ty!(
    if_let_or,
    "let n = 1;",
    "if let 0 | 1 = n { 1 } else { 0 }" => Int,
);

test_ty!(
    if_let_null_pattern,
    "let a: int? = 0;",
    "if let b? = a { b } else { 0 }" => Int,
);

// Simple literals
test_ty!(unit, "()" => Unit);
test_ty!(bools, "true" => Bool, "false" => Bool);
test_ty!(int, "1" => Int);
test_ty!(float, "0.1" => Float);
test_ty!(hex, "0xffffff" => Int);
test_ty!(string, r#""foo""# => Str);
test_ty!(fstring_literal_only, r#"f"foo""# => Str);
test_ty!(
    fstring_with_expr,
    "let a = 0;",
    r#"f"a is {a}""# => Str,
);
test_fail!(fstring_unknown_ident, r#"let a = f"{nope}";"#);

// Logical
test_ty!(
    logical,
    "true && false" => Bool,
    "true || false" => Bool,
);
test_fail!(logical_on_int, "let a = 0 && 0;");

// postfix accesses chain onto block expressions (the `while {...}.flatten()` parse bug)
test_ty!(chain_after_match, "match 0 { _ => (1, 2) }.0" => Int);
test_ty!(chain_after_if, "if true { (1, 2) } else { (3, 4) }.0" => Int);
test_ty!(chain_after_loop, "loop { break (1, 2); }.0" => Int);

// Null Coalescence
test_ty!(coalescence, "let a: int? = null;", "a ?? 0" => Int);
test_ty!(
    nested_coalescence,
    "let a: int? = null;
     let b: int? = null;",
    "a ?? b ?? 0" => Int,
);
test_ty!(
    assign_coalesce_operator,
    "let a: int? = null;
    a ??= 1;",
    "a" => option!(Int)
);
test_fail!(invalid_null_coalescence_target, "let a = 0 ?? 0;");

test_ty!(coalesce_null_left, "null ?? 0" => Int);

test_ty!(
    unary,
    "let a = 0;
    let b = false;",
    "+a" => Int,
    "-a" => Int,
    "~a" => Int,
    "!b" => Bool,
);
test_fail!(unary_minus_on_bool, "let a = -true;");

// In expressions
test_ty!(
    in_array,
    "let a = [0, 1, 2];",
    "0 in a" => Bool,
    "0 !in a" => Bool,
);

test_ty!(
    in_str,
    r#""hello" in "say hello world""# => Bool,
    r#""hello" !in "say hello world""# => Bool,
);

test_ty!(
    in_dict,
    r#"let a = ~{ foo = 0 };"#,
    r#""foo" in a"# => Bool,
    r#""foo" !in a"# => Bool,
);

test_fail!(
    in_does_not_search_values,
    "let a = ~{ foo = 0}; let b = 0 in a;"
);

// ice prevention
test_fail!(div_by_zero, "const DIV_ZERO: int = 1 ~/ 0;");
test_fail!(mod_by_zero, "const MOD_ZERO: int = 1 % 0;");
test_fail!(negative_shift, "const SHIFT_NEG: int = 1 << -1;");
test_fail!(
    giant_hex_const,
    "const GIANT_HEX: int = 0xffffffffffffffffffff;"
);
test_fail!(giant_hex_let, "let a = 0xffffffffffffffffff;");
test_fail!(tuple_ident_index, "let a = (0, 1); let b = a.foo;");
test_success!(
    add_overflow,
    "const ADD_OVER: int = 9223372036854775807 + 1;"
);
test_success!(
    mul_overflow,
    "const MUL_OVER: int = 9223372036854775807 * 2;"
);
test_success!(shift_overflow, "const SHIFT_OVER: int = 1 << 100;");
test_success!(const_tuple, "const T: (int, int) = (1, 2);");
test_success!(
    const_struct,
    "struct S { x: int }
     const S0: S = S { x = 5 };"
);
test_success!(const_dict, "const D: ~{int} = ~{ a = 1, b = 2 };");

// TypeMismatch shapes
test_fail!(
    let_annotation_mismatch,
    "let a: int = \"str\";",
    "let a: bool = 0;",
    "let a: float = 0;",
);
test_fail!(nested_array_element_mismatch, "let a: [[int]] = [[\"x\"]];");
test_fail!(dict_value_mismatch, "let a: ~{int} = ~{ k = \"x\" };");
test_fail!(tuple_element_mismatch, "let a: (int, str) = (0, 0);");
test_fail!(
    tuple_arity_mismatch,
    "let a: (int, int, int) = (0, 1);",
    "let a: (int, int) = (0, 1, 2);",
);
test_fail!(index_assign_mismatch, "let a = [0]; a[0] = \"x\";");
test_ty!(string_square_index, "let s = \"hi\";", "s[0]" => Str);
test_fail!(for_iter_elem_used_wrong, "for x in [0] { let y: str = x; }");
test_fail!(while_cond_non_bool, "while 5 {}");
test_fail!(logical_rhs_non_bool, "let x = true && 2;");
test_fail!(
    chained_if_arms_mismatch,
    "let a = if true { 0 } else if true { 1 } else { \"x\" };"
);
test_fail!(
    loop_yield_arms_mismatch,
    "loop { break 0; break \"x\"; }",
    "loop { collect 0; collect \"x\"; }",
);

// NotFound positions
test_fail!(notfound_type_annotation, "let a: Nope = 0;");
test_fail!(notfound_in_array, "let a = [missing];");

// IfNeedsElse: a value-yielding if/chain with no else
test_fail!(if_yields_str_without_else, "let x = if true { \"a\" };");
test_fail!(
    if_chain_yields_without_else,
    "let x = if true { 0 } else if false { 1 };"
);

// InvalidEvaluation
test_fail!(eval_str_div, "let x = \"a\" / \"b\";");
test_fail!(eval_bool_plus, "let x = true + false;");
test_fail!(eval_array_plus, "let x = [0] + [1];");
test_fail!(eval_dict_plus, "let x = ~{a=0} + ~{b=1};");
test_fail!(eval_float_bitor, "let x = 1.0 | 1.0;");
test_fail!(eval_shift_by_float, "let x = 1 << 1.0;");

// InvalidUnary
test_fail!(unary_neg_str, "let x = -\"hello\";");
test_fail!(unary_not_int, "let x = !5;");

// comparison / iter target
test_fail!(compare_structs, "struct S {} let c = S {} > S {};");
test_fail!(
    iter_over_non_iterable,
    "for x in true {}",
    "for x in 1.0 {}",
    "struct S {} for x in S {} {}",
);

// InvalidAccess / InvalidInTarget
test_fail!(
    square_index_on_non_indexable,
    "let x = 5; let y = x[0];",
    "let x = true; let y = x[0];",
);
test_fail!(
    in_on_non_collection,
    "let x = 5 in 10;",
    "let x = true in false;",
);

// loop/flow context errors beyond top-level
test_fail!(collect_in_fn_no_loop, "fn f() { collect 0; }");
test_fail!(continue_in_fn_no_loop, "fn f() { continue; }");
test_fail!(break_value_at_top_level, "break 5;");
test_fail!(return_value_at_top_level, "return 5;");
test_fail!(
    break_value_with_collection,
    "while true { collect 0; break 5; }",
    "for x in [] { collect 0; break 5; }",
);

test_ty!(
    optional_string_square_index,
    "let s: str? = null;",
    "s?[0]" => option!(Str),
);
test_fail!(
    str_square_index_by_str,
    "let s = \"abc\"; let c = s[\"k\"];"
);
test_fail!(string_index_assign, "let s = \"abc\"; s[0] = \"x\";");
