#[macro_use]
mod vm_test_utils;

use vm::Captured::*;

test_vm!(
    literals,
    "true" => Bool(true),
    "false" => Bool(false),
    "1" => Int(1),
    "0xf" => Int(15),
    "0.5" => Float(0.5),
    "\"foo\"" => str!("foo"),
    "[0]" => array!(Int(0)),
    "(0,)" => array!(Int(0)),
    "~{ foo = 0 }" => dict!(~{
        foo = Int(0)
    }),
);

test_vm!(
    null_value,
    "let n: int? = null;",
    "n" => Null
);

test_vm!(
    unit_is_runtime_null,
    "fn foo() {}",
    "foo()" => Null
);

test_vm!(
    grouping,
    "(0)" => Int(0)
);

test_vm!(
    block,
    "{ 0 }" => Int(0),
    "{ let a = 0; { let b = 1; a + b } }" => Int(1),
);

test_vm!(
    scope_shadows_inner,
    "let a = 0;
    {
        let a = 5;
    }",
    "a" => Int(0),
);

test_vm!(
    scope_assigns_outer,
    "let a = 0;
    {
        a = 5;
    }",
    "a" => Int(5),
);

test_vm!(
    block_propagates_compound_assign,
    "let a = 10;
     {
        a += 5;
     }",
    "a" => Int(15),
);

test_vm!(
    equality,
    "0 == 0" => Bool(true),
    "0 == 1" => Bool(false),
    "0 != 1" => Bool(true),
    "0 > 0" => Bool(false),
    "0 >= 0" => Bool(true),
    "0 < 0" => Bool(false),
    "0 <= 0" => Bool(true),
    r#""foo" == "foo""# => Bool(true),
    r#""foo" == "bar""# => Bool(false),
    r#""good_song" != "logical""# => Bool(true),
);

// bool operands via params (so they aren't const-folded) exercise the BoolEq/BoolNe ops
test_vm!(
    bool_equality,
    "fn eq(a: bool, b: bool) -> bool { a == b }
     fn ne(a: bool, b: bool) -> bool { a != b }",
    "eq(true, true)" => Bool(true),
    "eq(true, false)" => Bool(false),
    "ne(true, false)" => Bool(true),
    "ne(false, false)" => Bool(false),
);

test_vm!(
    arithmetic,
    r#""foo" + "bar""# => str!("foobar"),
    "0.5 + 0.5" => Float(1.0),
    "0.5 - 0.5" => Float(0.0),
    "0.5 * 2.0" => Float(1.0),
    "2.0 / 2.0" => Float(1.0),
    "5.0 ~/ 2.0" => Float(2.0),
    "0 + 1" => Int(1),
    "0 - 1" => Int(-1),
    "0 * 2" => Int(0),
    "5 ~/ 2" => Int(2),
    "5 % 2" => Int(1),
    "-5 + 3" => Int(-2),
    "5 + -3" => Int(2),
    "-1.0 + 2.0" => Float(1.0),
);

// `/` is always float, even on two ints that divide evenly
test_vm!(
    slash_always_float,
    "10 / 5" => Float(2.0),
    "1 / 4" => Float(0.25),
    "10 / 4" => Float(2.5),
);

test_vm!(
    slash_result_is_float_typed,
    "let r = 10 / 2;",
    "r + 0.5" => Float(5.5),
);

// `~/` floor-divides within the operand domain: int stays int, float stays float
test_vm!(
    floordiv_stays_in_domain,
    "5 ~/ 2" => Int(2),
    "7 ~/ 3" => Int(2),
    "5.0 ~/ 2.0" => Float(2.0),
);

test_vm!(
    int_float_mixed,
    "1 + 2.0" => Float(3.0),
    "2.0 + 1" => Float(3.0),
    "1 < 2.0" => Bool(true),
    "2.0 > 1" => Bool(true),
    "1.0 == 1" => Bool(true),
    "1 == 1.0" => Bool(true),
);

// whole floats print without a trailing `.0` (heap.rs uses `{f}`)
test_vm!(
    whole_float_prints_bare,
    "2.0 + 2.0" => Float(4.0),
    "8.0 / 2.0" => Float(4.0),
    "3.0 * 3.0" => Float(9.0),
);

// float div-by-zero yields inf, no fault
test_vm!(
    float_div_zero_is_inf,
    "let r = 1.0 / 0.0;",
    "r > 1.0e9 || r == r" => Bool(true),
);

// `%` sign follows the lhs
test_vm!(
    mod_sign_follows_lhs,
    "5 % 2" => Int(1),
    "-5 % 3" => Int(-2),
    "5 % -3" => Int(2),
    "-5 % -3" => Int(-2),
    "-5.0 % 3.0" => Float(-2.0),
    "5.0 % -3.0" => Float(2.0),
);

test_vm!(
    tuple_arithmetic,
    r#"("foo", "fizz") + ("bar", "buzz")"# => array!(str!("foobar"), str!("fizzbuzz")),
    "(0.5, 9.0) + (0.5, 1.0)" => array!(Float(1.0), Float(10.0)),
    "(0.5, 10.0) - (0.5, 6.0)" => array!(Float(0.0), Float(4.0)),
    "(0.5, 5.0) * (2.0, 4.0)" => array!(Float(1.0), Float(20.0)),
    "(2.0, 3.0) / (2.0, 0.5)" => array!(Float(1.0), Float(6.0)),
    "(5.0, 5.0) ~/ (2.0, 3.0)" => array!(Float(2.0), Float(1.0)),
    "(5, 5) % (2, 3)" => array!(Int(1), Int(2)),
    "(0, 0) + (1, 2)" => array!(Int(1), Int(2)),
    "(5, 4) - (1, 2)" => array!(Int(4), Int(2)),
    "(0, 1) * (2, 5)" => array!(Int(0), Int(5)),
    "(2, 4) / (2, 1)" => array!(Float(1.0), Float(4.0)),
    "(5, 10) ~/ (2, 2)" => array!(Int(2), Int(5)),
    "(5, 10) % (2, 1)" => array!(Int(1), Int(0)),
    "(10, 10) * 2" => array!(Int(20), Int(20)),
    "(10, 10) / 2" => array!(Float(5.0), Float(5.0)),
);

test_vm!(
    unary,
    "-(1.1)" => Float(-1.1),
    "+1.0" => Float(1.0),
    "-1" => Int(-1),
    "+1" => Int(1),
    "+(-1)" => Int(1),
    "!true" => Bool(false),
);

test_vm!(
    bit_not,
    "~0" => Int(-1),
    "~5" => Int(-6),
    "~-1" => Int(0),
);

test_vm!(
    bit_ops_combined,
    "(0xff & 0xf0) | 0x0f" => Int(0xff),
    "0xff ^ 0xff" => Int(0),
);

test_vm!(
    shifts,
    "1 << 4" => Int(16),
    "16 >> 2" => Int(4),
    "1 << 0" => Int(1),
);

test_vm!(
    hex_literals,
    "0xff" => Int(255),
    "0xFF" => Int(255),
    "0xFFFF" => Int(65535),
    "0x7FFFFFFFFFFFFFFF" => Int(9223372036854775807),
    "-0xff" => Int(-255),
);

test_vm!(
    underscore_separators,
    "1_000" => Int(1000),
    "1_000_000" => Int(1000000),
    "1_000.5" => Float(1000.5),
);

test_vm!(
    int_boundaries,
    "9223372036854775807" => Int(9223372036854775807),
);

test_vm!(
    int_min_via_subtraction,
    "let big = 9223372036854775807;
     let m = -big - 1;",
    "m" => Int(-9223372036854775808),
);

test_vm!(
    assignment,
    "let a = 0;
    a = 1;",
    "a" => Int(1),
);

test_vm!(
    compound_assignment,
    "let a = 10;
     a -= 3;
     a *= 2;
     a %= 5;",
    "a" => Int(4),
);

test_vm!(
    plus_equal,
    "let a = 0;
    a += 1;",
    "a" => Int(1)
);

test_vm!(
    plus_equal_str,
    "let a = \"foo\";
    a += \"bar\";",
    "a" => str!("foobar")
);

test_vm!(
    bool_precedence,
    "true && true || false" => Bool(true),
    "false || true && true" => Bool(true),
    "true && false" => Bool(false),
);

// `&&` / `||` don't evaluate their rhs once the result is settled
test_vm!(
    bool_short_circuits,
    "fn boom() -> bool { raise_it() }
     fn raise_it() -> bool { true }
     let a = false && boom();
     let b = true || boom();",
    "a" => Bool(false),
    "b" => Bool(true),
);

test_vm!(
    fstring_literal_only,
    r#"f"hello""# => str!("hello"),
);

test_vm!(
    fstring_with_int,
    r#"let a = 0;"#,
    r#"f"{a} and {1 + 1}""# => str!("0 and 2"),
);

test_vm!(
    fstring_with_str,
    r#"let name = "world";"#,
    r#"f"hello {name}!""# => str!("hello world!"),
);

test_vm!(
    fstring_brace_escapes,
    "let n = 3;",
    "f\"{{{n}}}\"" => str!(r#"{3}"#),
    "f\"{{x}}\"" => str!(r#"{x}"#),
);

// `\{` / `\}` are an alternate brace escape -- the form `fodder/ty.mim` uses for `~\{ T \}`
test_vm!(
    fstring_backslash_brace_escapes,
    r#"let t = "int";"#,
    r#"f"\{ {t} \}""# => str!("{ int }"),
    r#"f"~\{ {t} \}""# => str!("~{ int }"),
);

// the two brace-escape forms mix freely in one string
test_vm!(
    fstring_brace_escape_forms_mix,
    r#"f"\{ {{ \} }}""# => str!("{ { } }"),
);

test_vm!(
    array_membership,
    "let a = [0];",
    "0 in a" => Bool(true),
    "0 !in a" => Bool(false),
);
test_vm!(
    dict_membership,
    "let a = ~{ foo = 0 };",
    "\"foo\" in a" => Bool(true),
    "\"foo\" !in a" => Bool(false),
);
test_vm!(
    str_membership,
    "let a = \"foo\";",
    "\"f\" in a" => Bool(true),
    "\"f\" !in a" => Bool(false),
);

// regular (non-f) strings keep `\{` / `\}` verbatim -- braces aren't special outside f-strings
test_vm!(
    plain_string_keeps_backslash_braces,
    r#""\{ hi \}""# => str!(r#"\{ hi \}"#),
);

test_vm!(
    array_structural_inequality,
    "[1, 2, 3] != [1, 2, 3]" => Bool(false),
);

test_vm!(
    array_structural_inequality_differs,
    "[1, 2, 3] != [1, 2, 4]" => Bool(true),
);

test_vm!(
    nested_array_structural_eq,
    "[[1, 2], [3, 4]] != [[1, 2], [3, 4]]" => Bool(false),
);

test_vm!(
    dict_structural_eq,
    "~{ a = 1, b = 2 } == ~{ a = 1, b = 2 }" => Bool(true),
);

// order-independent
test_vm!(
    dict_structural_eq_reordered,
    "~{ a = 1, b = 2 } == ~{ b = 2, a = 1 }" => Bool(true),
);

test_vm!(
    dict_structural_neq,
    "~{ a = 1 } == ~{ a = 2 }" => Bool(false),
);

test_vm!(
    instance_structural_eq,
    "struct P { x: int, y: int }
     let a = P { x = 1, y = 2 };
     let b = P { x = 1, y = 2 };",
    "a == b" => Bool(true),
);

test_vm!(
    instance_structural_neq,
    "struct P { x: int, y: int }
     let a = P { x = 1, y = 2 };
     let b = P { x = 1, y = 9 };",
    "a == b" => Bool(false),
);

// nested struct fields compare recursively
test_vm!(
    nested_instance_structural_eq,
    "struct Inner { v: int }
     struct Outer { i: Inner }
     let a = Outer { i = Inner { v = 5 } };
     let b = Outer { i = Inner { v = 5 } };",
    "a == b" => Bool(true),
);
