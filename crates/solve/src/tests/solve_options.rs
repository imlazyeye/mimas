use crate::components::Ty::*;

test_ty!(
    explicate,
    "let a: int? = null;
     let b: bool? = false;",
    "a" => option!(Int),
    "b" => option!(Bool),
);
test_ty!(
    option_array,
    "let a: [int]? = null;",
    "a" => option!(array!(Int)),
);
test_ty!(
    option_dict,
    "let a: ~{int}? = null;",
    "a" => option!(dictionary!(Int)),
);
test_ty!(
    array_of_optional,
    "let a: [int?] = [0, null, 1];",
    "a" => array!(option!(Int)),
);
test_ty!(
    dict_of_optional,
    "let a: ~{int?} = ~{ x = 0, y = null };",
    "a" => dictionary!(option!(Int)),
);
test_ty!(
    coerce_from_if,
    "if true { 0 } else { null }" => option!(Int)
);
test_ty!(
    coerce_from_loop,
    "loop { break 0; break null; }" => option!(Int)
);
test_ty!(
    flattening,
    "let a = if true { 0 } else { null };
    let b = if true { a } else { null };
    let c = if true { b } else { null };",
    "c" => option!(Int)
);
test_ty!(unwrap, "let a: int? = 0;", "a!" => Int);

// A trailing `?` postfix after a call or square access is accepted by the parser as a no-op.
// It's the syntactic form `expr?` (vs the option-chaining `?.field` / `?[key]` infix tokens),
// reserved for future option-aware chaining; for now it's just a parse-accepted nullable hint.
test_ty!(
    postfix_hook_after_square,
    "let a: [int?] = [1, null, 3];",
    "a[1]?" => option!(Int),
);
test_ty!(
    postfix_hook_after_call,
    "fn maybe() -> int? { 5 }",
    "maybe()?" => option!(Int),
);
test_ty!(
    unwrap_through_dot,
    "struct Foo { x: int }
     let foo: Foo? = Foo { x = 0 };",
    "foo!.x" => Int,
);

test_ty!(
    t_against_option,
    "let a: int = 0;
     let b: int? = a;",
    "b" => option!(Int)
);
test_fail!(unwrap_non_option, "let a = 0; let b = a!;");
test_fail!(
    option_against_t,
    "let a: int? = null;
     let b: int = a;"
);
test_fail!(
    coalesce_mismatched_types,
    "let a: int? = null; let b = a ?? \"default\";"
);

test_fail!(
    null_is_permanent,
    "let a = null;
    if true {
        a = 1;
    } else {
        a = null;
    };"
);

test_fail!(
    option_ordering_error,
    "let a: int = 0;
    let b: int? = null;
    let c = a > b;"
);

test_fail!(
    option_ordering_error_reversed,
    "let a: int = 0;
    let b: int? = null;
    let c = b > a;"
);

test_ty!(
    match_option_null_and_bind_exhaustive,
    "let x: int? = 0;",
    "match x { null => 0, n? => n }" => Int,
);

test_ty!(
    match_option_null_and_wildcard_exhaustive,
    "let x: int? = 0;",
    "match x { null => 0, _ => 1 }" => Int,
);

test_ty!(
    match_option_with_panic_terminator,
    "let x: int? = 0;",
    "match x { n? => n, ! }" => Int,
);

test_fail!(
    match_option_only_bind_non_exhaustive,
    "let x: int? = 0; let y = match x { n? => n };"
);

test_fail!(
    match_option_only_null_non_exhaustive,
    "let x: int? = 0; let y = match x { null => 0 };"
);

test_fail!(coalesce_assign_on_non_option, "let a = 0; a ??= 1;");
