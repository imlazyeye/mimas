use crate::components::Ty::*;

// Basic headers
test_ty!(
    simple,
    "fn foo() {}",
    "foo" => func!(() -> Unit)
);
test_ty!(
    return_int,
    "fn foo() -> int { 0 }",
    "foo" => func!(() -> Int)
);
test_ty!(
    return_optional,
    "fn try_get() -> int? { null }",
    "try_get" => func!(() -> option!(Int)),
);
test_ty!(
    multiple_typed_params,
    "fn foo(a: int, b: str, c: bool) -> int { a }",
    "foo" => func!((Int, Str, Bool) -> Int)
);
test_ty!(
    infer_from_default,
    "fn foo(a=0) -> int { a }",
    "foo" => func!((Int) -> Int)
);
test_ty!(
    default_match_annotation,
    "fn foo(a: int = 0) {}",
    "foo" => func!((Int) -> Unit)
);
// test_ty!(
//     infer_unannotated_params,
//     "fn identity(a) -> int { a }
//     fn add_one(a) -> int { a + 1 }
//     fn suffix(a) -> str { a + \"!\" }
//     fn first(a) -> int { a[0] }
//     fn observe(a) { let ok = a == 0; }",
//     "identity" => func!((Int) -> Int),
//     "add_one" => func!((Int) -> Int),
//     "suffix" => func!((Str) -> Str),
//     "first" => func!((array!(Int)) -> Int),
//     "observe" => func!((Int) -> Unit)
// );

// Control flow
test_ty!(
    return_unit,
    "fn foo() { return; }",
    "foo" => func!(() -> Unit)
);
test_ty!(
    return_value,
    "fn foo() -> int { return 0; }",
    "foo" => func!(() -> Int)
);
test_ty!(
    two_path_return,
    "fn foo() -> int {
        if true {
            return 0;
        } else {
            return 0;
        }
    }",
    "foo" => func!(() -> Int)
);
test_ty!(
    early_return,
    "fn foo() -> int {
        if true {
            return 0;
        }
        1
    }",
    "foo" => func!(() -> Int)
);
test_ty!(
    unit_return_without_all_paths,
    "fn foo() {
        if true {
            return;
        }
    }",
    "foo" => func!(() -> Unit)
);
test_ty!(
    never_coerces_in_binding,
    "fn foo(cond: bool) -> int {
        let a: int = if cond { 0 } else { return 1; };
        a
    }",
    "foo" => func!((Bool) -> Int)
);

// Identity
test_ty!(
    identity,
    "struct Foo { x: int }
    impl Foo {
        fn bar(self) -> int { self.x }
    }
    let foo = Foo { x = 0 };",
    "foo.bar()" => Int
);

test_ty!(
    nested_identity,
    "struct Foo { x: int }
    impl Foo {
        fn bar(self) -> int { self.x }

        fn fizz(self) -> int { self.bar() }
    }
    let foo = Foo { x = 0 };",
    "foo.fizz()" => Int
);

test_ty!(
    method_with_self_and_args,
    "struct Foo { x: int }
     impl Foo {
         fn add(self, n: int) -> int { self.x + n }
     }
     let foo = Foo { x = 1 };",
    "foo.add(2)" => Int
);

// Calls
test_ty!(
    call,
    "fn foo() {}",
    "foo()" => Unit,
);
test_ty!(
    call_with_positional_int,
    "fn foo(a: int) {}",
    "foo(1)" => Unit,
);
test_ty!(
    call_with_return,
    "fn foo() -> int { 0 }",
    "foo()" => Int,
);
test_ty!(
    call_with_argument_return,
    "fn foo(a: int) -> int { a }",
    "foo(0)" => Int,
);
test_ty!(
    call_with_default_omitted,
    "fn foo(a: int = 0) -> int { a }",
    "foo()" => Int,
    "foo(5)" => Int,
);
test_ty!(
    call_with_multiple_defaults,
    "fn foo(a=1, b=2, c=3) -> int { a + b + c }",
    "foo()" => Int,
    "foo(10)" => Int,
    "foo(10, 20)" => Int,
    "foo(10, 20, 30)" => Int,
);
test_ty!(
    recursive_fn,
    "fn foo () { foo() }",
    "foo" => func!(() -> Unit)
);
test_ty!(
    mutually_recursive_fns,
    "fn foo(n: int) -> int { if n == 0 { 0 } else { bar(n - 1) } }
     fn bar(n: int) -> int { if n == 0 { 0 } else { foo(n - 1) } }",
    "foo" => func!((Int) -> Int),
    "bar" => func!((Int) -> Int)
);
test_ty!(
    func_out_of_order,
    "fn foo() { bar() }
    fn bar() {}",
    "foo" => func!(() -> Unit),
    "bar" => func!(() -> Unit),
);
test_ty!(
    func_before_struct,
    "fn foo() -> Bar { Bar {} }
    struct Bar;",
    "foo" => func!(() -> query!(Bar))
);

// Closures
test_ty!(
    closure,
    "|| {}" => func!(() -> Unit),
);
test_ty!(
    closure_with_annotated_arg,
    "|a: int| {}" => func!((Int) -> Unit),
);
test_ty!(
    closure_with_multiple_args,
    "|a: int, b: str| {}" => func!((Int, Str) -> Unit),
);
test_ty!(
    closure_returns_int,
    "|a: int| { a }" => func!((Int) -> Int),
);
test_ty!(
    closure_with_return_type_annotation,
    "|a: int| -> int { a }" => func!((Int) -> Int),
);
test_ty!(
    named_args_all_by_name,
    "fn foo(a: int, b: int, c: int = 0) -> int { a + b + c }",
    "foo(b=2, a=1)" => Int
);
test_ty!(
    named_args_call,
    "fn foo(a: int, b: int = 0, c: int = 0) -> int { a + b + c }",
    "foo(1, c=3, b=2)" => Int
);

// Violations
test_fail!(non_explicate_return_ty, "fn foo() { 0 }");
// test_fail!(uninferrable_param_echo, "fn echo(a) { a }");
// test_fail!(uninferrable_param_unit_body, "fn foo(a) {}");
// test_fail!(uninferrable_param_explicit_return, "fn foo(a) -> int { 0 }");
// test_fail!(uninferrable_second_param, "fn foo(a, b) -> int { a }");
test_fail!(default_mismatch_annotation, "fn foo(a: str = 0) {}");
test_fail!(yield_wrong_ty, "fn foo() -> int { null }");
test_fail!(
    return_without_needed_block_value,
    "fn foo() -> int {
        if true {
            return 0;
        }
    }"
);
test_fail!(
    not_all_paths_return,
    "fn foo() -> int {
        if true {
            if true {
                return 0;
            }
        } else {
            return 0;
        }
    }"
);
test_fail!(
    unit_return_with_value_yield,
    "fn foo() -> int {
        if true {
            return;
        }
        0
    }"
);
test_fail!(extra_argument, "fn foo() {}; foo(0);");
test_fail!(missing_argument, "fn foo(a: int) {}; foo();");
test_fail!(mismatched_argument, "fn foo(a: int) {}; foo(true);");
test_fail!(
    optional_params_before_required,
    "fn foo(a: int, b: int = 0, c: int) {}"
);
test_fail!(
    named_call_missing_required,
    "fn foo(a: int, c: int, b: int = 0) {}
    foo(0, b=3);"
);

test_fail!(fn_does_not_hoist_local, "let a = 0; fn c() -> int { a }");
test_ty!(closure_captures_local, "let a = 0; let b = || a;", "b" => func!(() -> Int));
test_ty!(
    fn_sees_const_and_items,
    "const A = 0;
    fn helper() -> int { 0 }
    fn c() -> int { A + helper() }",
    "c" => func!(() -> Int)
);
test_ty!(
    const_in_fn,
    "fn foo() -> int { const A = 0; A }",
    "foo()" => Int,
);

test_ty!(
    const_used_before_decl_in_fn,
    "fn foo() -> int { let x = A; const A = 7; x }",
    "foo()" => Int,
);

test_ty!(
    const_used_before_decl_in_block,
    "fn foo() -> int { { let x = A; const A = 7; x } }",
    "foo()" => Int,
);

test_ty!(
    function_type_annotation,
    "fn helper(a: int, b: int) -> int { a + b }
     let f: (int, int) -> int = helper;",
    "f(1, 2)" => Int,
);

test_ty!(
    pass_fn_to_fn,
    "fn helper(n: int) -> int { n + 1 }
     fn apply(f: (int) -> int, n: int) -> int { f(n) }",
    "apply(helper, 5)" => Int,
);

test_ty!(
    pass_closure_to_fn,
    "fn apply(f: (int) -> int, n: int) -> int { f(n) }",
    "apply(|n| n + 1, 5)" => Int,
);
test_ty!(
    pass_closure_str_param,
    "fn run(f: (str) -> str) -> str { f(\"hello\") }",
    "run(|s| s + \"!\")" => Str,
);
test_success!(
    closure_arg_used_twice,
    "fn twice(f: (int) -> int, n: int) -> int { f(f(n)) }
     let r = twice(|n| n * n, 3);"
);

test_fail!(
    closure_return_mismatch,
    "fn apply(f: (int) -> int, n: int) -> int { f(n) }
     let r = apply(|n| \"not an int\", 10);"
);
test_fail!(
    closure_param_type_mismatch,
    "fn run(f: (str) -> int) -> int { f(\"hi\") }
     let r = run(|n| n + 1);"
);

// NotFound / NotCallable in call position
test_fail!(call_undefined, "let a = nope();");
test_fail!(
    not_callable,
    "let x = 5; let y = x();",
    "let x = \"a\"; let y = x();",
);

// arity: too few / too many positional args
test_fail!(missing_one_of_two_args, "fn f(a: int, b: int) {} f(1);");
test_fail!(
    extra_args,
    "fn f() {} f(1, 2, 3);",
    "fn f(a: int) {} f(1, 2);",
);

// named-arg errors
test_fail!(unknown_named_argument, "fn f(a: int) {} f(b = 1);");
test_fail!(duplicate_named_argument, "fn f(a: int) {} f(a = 1, a = 2);");

// required param after an optional one
test_fail!(required_after_optional, "fn f(a: int = 0, b: int) {}");

// fn-type annotation mismatch on a let binding
test_fail!(
    fn_type_annotation_mismatch,
    "fn h(a: int) -> int { a } let f: (str) -> int = h;"
);

// NotAllPathsReturn: body falls off the end without a value
test_fail!(
    not_all_paths_return_empty_body,
    "fn f() -> int { let x = 0; }"
);

// NonConstDefault: a default expr that isn't const-foldable
test_fail!(
    non_const_default_call,
    "fn h() -> int { 0 } fn f(a: int = h()) {}"
);

// self / nested-fn structural errors
test_fail!(
    self_outside_method_positions,
    "let x = self;",
    "fn f() -> int { self.x }",
);
test_fail!(
    nested_fn,
    "fn outer() { fn inner() {} }",
    "fn outer() { { fn inner() {} } }",
);
