#[macro_use]
mod vm_test_utils;

use vm::Captured::*;

test_vm!(
    array_access,
    "let a = [0, 1, 2];",
    "a[0]" => Int(0)
);

test_vm!(
    array_chained_access,
    "let a = [[[0]]];",
    "a[0][0][0]" => Int(0)
);

test_vm!(
    array_access_option,
    "let a = [[0], null];",
    "a[0]" => array!(Int(0)),
    "a[0]?[0]" => Int(0),
    "a[1]" => Null,
    "a[1]?[0]" => Null,
);

test_vm!(
    array_access_unwrap,
    "let a: [[int]?] = [[1]];",
    "a[0]![0] * 5" => Int(5),
);

test_vm!(
    dict_index_present_and_missing,
    "let d = ~{ a = 1, b = 2 };",
    "d[\"a\"]" => Int(1),
    "d[\"z\"]" => Null,
);

test_vm!(
    dict_taint_propagates,
    "let a = [ ~{ foo = [[7]] } ];",
    "a[0][\"foo\"]?[0][0]" => Int(7),
);

// a `?` per dict reached through: two dict hops -> two `?`. the second `?["c"]` short-circuits
// when `["b"]` misses, so the chain comes out null instead of faulting.
test_vm!(
    dict_two_questions,
    "let nested: ~{~{~{int}}} = ~{ a = ~{ b = ~{ c = 9 } } };
     let hole: ~{~{~{int}}} = ~{ a = ~{} };",
    "nested[\"a\"]?[\"b\"]?[\"c\"]" => Int(9),
    "hole[\"a\"]?[\"b\"]?[\"c\"]" => Null,
);

test_vm!(
    tuple_access,
    "let a = (0, 1);",
    "a.0" => Int(0),
    "a.1" => Int(1)
);

test_vm!(
    in_array,
    "let a = [1, 2, 3];",
    "2 in a" => Bool(true),
    "4 in a" => Bool(false),
);

test_vm!(
    in_dict,
    r#"let d = ~{ foo = 0 };"#,
    r#""foo" in d"# => Bool(true),
    r#""bar" in d"# => Bool(false),
);

test_vm!(
    in_str,
    r#""hello world" in "say hello world""# => Bool(true),
    r#""xyz" in "hello""# => Bool(false),
);

test_vm!(
    array_of_dict_access,
    r#"let a = [~{ x = 1 }, ~{ x = 2 }];"#,
    r#"a[0]["x"]"# => Int(1),
    r#"a[1]["x"]"# => Int(2),
);

test_vm!(
    let_tuple_destructure,
    "let (a, b) = (1, 2);",
    "a" => Int(1),
    "b" => Int(2),
);

test_vm!(
    let_tuple_destructure_mixed_types,
    r#"let (s, n) = ("hi", 42);"#,
    "s" => str!("hi"),
    "n" => Int(42),
);

test_vm!(
    let_tuple_destructure_nested,
    "let ((a, b), c) = ((1, 2), 3);",
    "a" => Int(1),
    "b" => Int(2),
    "c" => Int(3),
);

test_vm!(
    for_tuple_destructure,
    "let pairs = [(1, 2), (3, 4), (5, 6)];",
    "for (a, b) in pairs collect a + b" => array!(Int(3), Int(7), Int(11)),
);

test_vm!(
    for_tuple_destructure_mixed_types,
    r#"let entries = [("a", 1), ("b", 2)];"#,
    r#"for (k, v) in entries collect f"{k}={v}""#
        => array!(str!("a=1"), str!("b=2")),
);

test_vm!(
    for_tuple_destructure_nested,
    "let nested = [((1, 2), 3), ((4, 5), 6)];",
    "for ((a, b), c) in nested collect a + b + c" => array!(Int(6), Int(15)),
);

test_vm!(
    array_by_reference,
    "let a = [0]; let b = a; b[0] = 1;",
    "a[0]" => Int(1)
);
