use crate::components::Ty::*;

test_ty!(
    result_annotation,
    "let a: int! = 0;",
    "a" => result!(Int),
);

test_ty!(
    auto_wrap_ok,
    "fn foo() -> int! { 0 }",
    "foo()" => result!(Int),
);

test_ty!(
    raise_produces_never_in_fn,
    "fn foo() -> int! { raise \"bad\" }",
    "foo()" => result!(Int),
);

test_ty!(
    unwrap_result,
    "fn foo() -> int! { 0 }",
    "foo()!" => Int,
);

test_ty!(
    absolve_returns_inner,
    "fn foo() -> int! { 0 }",
    "foo() absolve |_| 0" => Int,
);

test_ty!(
    result_of_array,
    "let a: [int]! = [0];",
    "a" => result!(array!(Int)),
);

test_ty!(
    option_of_result,
    "let a: int!? = null;",
    "a" => option!(result!(Int)),
);

test_fail!(
    raise_outside_result_fn,
    "fn bad() -> int { raise \"no\" }",
    "fn f() { raise \"x\" }",
);

test_fail!(raise_outside_function, "raise \"no\";");

test_fail!(raise_non_string, "fn bad() -> int! { raise 0 }");

test_fail!(
    absolve_on_int,
    "let x = 5 absolve |_| 0;",
    "let x = \"a\" absolve |_| 0;",
);

test_fail!(
    absolve_on_option,
    "let a: int? = 0;
     let x = a absolve |_| 0;"
);

test_fail!(
    absolve_handler_type_mismatch,
    "fn foo() -> int! { 0 }
     let x = foo() absolve |_| \"oops\";"
);

// InvalidUnwrap: `!` on a non-result/non-option value
test_fail!(
    unwrap_non_result,
    "let a = \"x\"; let b = a!;",
    "let a = [0]; let b = a!;",
);
