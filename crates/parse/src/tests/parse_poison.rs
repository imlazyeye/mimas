use crate::{
    Parser,
    components::POISON,
    lex::Lexer,
    tests::utils::{label_text, parse, parse_to_strings},
};

/// Parses `$source` and checks every error (by message, in order) and every top-level stmt (by
/// its `Display`, with `"_"` matching anything).
macro_rules! test_recover {
    ($name:ident, $source:expr, errors: [$($error:expr),* $(,)?], stmts: [$($stmt:expr),* $(,)?] $(,)?) => {
        #[test]
        fn $name() {
            check_recovery($source, &[$($error),*], &[$($stmt),*]);
        }
    };
}

/// Each source reports exactly one error with the given message.
macro_rules! test_single_error {
    ($name:ident, $error:expr, $($source:expr),+ $(,)?) => {
        #[test]
        fn $name() {
            $(
                let source: &str = &$source;
                let (_, errors) = recover(source);
                pretty_assertions::assert_eq!(errors, [$error], "errors for `{}`", preview(source));
            )+
        }
    };
}

/// `$source` reports exactly one error, and its label sits on `$text`.
macro_rules! test_error_label {
    ($name:ident, $source:expr, $text:expr $(,)?) => {
        #[test]
        fn $name() {
            let (_, errors) = parse($source);
            assert_eq!(errors.len(), 1, "errors for `{}`", $source);
            assert_eq!(label_text($source, &errors[0]), $text);
        }
    };
}

/// Each source terminates without panicking and reports at least one error.
macro_rules! test_survives {
    ($name:ident, $($source:expr),+ $(,)?) => {
        #[test]
        fn $name() {
            $(
                let source: &str = &$source;
                let (_, errors) = recover(source);
                assert!(!errors.is_empty(), "`{}` should report an error", preview(source));
            )+
        }
    };
}

test_recover!(
    missing_let_value,
    "let a = ;
     let b = 2;",
    errors: ["expected expression"],
    stmts: ["let a = <poison>;", "let b = 2;"],
);

test_recover!(
    missing_binary_rhs,
    "let a = 1 + ;
     let b = 2;",
    errors: ["expected expression"],
    stmts: ["let a = 1 + <poison>;", "let b = 2;"],
);

test_recover!(
    missing_unary_operand,
    "let a = -;
     let b = 2;",
    errors: ["expected expression"],
    stmts: ["let a = -<poison>;", "let b = 2;"],
);

test_recover!(
    missing_assignment_value,
    "x = ;
     let b = 2;",
    errors: ["expected expression"],
    stmts: ["x = <poison>", "let b = 2;"],
);

test_recover!(
    empty_call_argument,
    "let a = foo(1, , 3);
     let b = 2;",
    errors: ["unexpected token"],
    stmts: ["let a = foo(1, 3);", "let b = 2;"],
);

test_recover!(
    missing_group_rhs,
    "let a = (1 + );
     let b = 2;",
    errors: ["expected expression"],
    stmts: ["let a = (1 + <poison>);", "let b = 2;"],
);

test_recover!(
    two_poisons_in_one_expr,
    "let a = (1 + ) * (2 - );
     let b = 2;",
    errors: ["expected expression", "expected expression"],
    stmts: ["let a = (1 + <poison>) * (2 - <poison>);", "let b = 2;"],
);

test_recover!(
    bad_dot_access,
    "let a = b.;
     let c = 3;",
    errors: ["invalid dot access"],
    stmts: ["let a = b.<poison>;", "let c = 3;"],
);

test_recover!(
    missing_path_member,
    "let a = b::;
     let c = 3;",
    errors: ["expected identifier"],
    stmts: ["let a = b::<poison>;", "let c = 3;"],
);

test_recover!(
    missing_closure_body,
    "let f = |x| ;
     let b = 2;",
    errors: ["expected expression"],
    stmts: ["let f = |x| <poison>;", "let b = 2;"],
);

test_recover!(
    poison_in_struct_literal,
    "let s = S { a = , b = 2 };
     let c = 3;",
    errors: ["expected expression"],
    stmts: ["let s = S { a = <poison>, b = 2 };", "let c = 3;"],
);

test_recover!(
    poison_in_block_yield,
    "let a = { let b = 1; b + };
     let c = 2;",
    errors: ["expected expression"],
    stmts: ["let a = {let b = 1;b + <poison> };", "let c = 2;"],
);

test_recover!(
    unfinished_dot_access_before_let,
    "let a = foo.
     let b = 2;",
    errors: ["invalid dot access"],
    stmts: ["let a = foo.<poison>;", "let b = 2;"],
);

test_recover!(
    unfinished_let_before_let,
    "let a =
     let b = 2;",
    errors: ["expected expression"],
    stmts: ["let a = <poison>;", "let b = 2;"],
);

test_recover!(
    unfinished_call_before_let,
    "foo(1,
     let b = 2;",
    errors: ["expected token"],
    stmts: ["foo(1)", "let b = 2;"],
);

test_recover!(
    unfinished_struct_before_fn,
    "struct Foo
     fn main() {}",
    errors: ["expected token"],
    stmts: ["struct Foo {  }", "fn main() {}"],
);

test_recover!(
    receiver_after_the_first_param,
    "fn f(a, self) {}
     let x = 1;",
    errors: ["expected identifier"],
    stmts: ["_", "let x = 1;"],
);

test_recover!(
    unfinished_params_before_fn,
    "fn f(a: int,
     fn g() {}",
    errors: ["expected token"],
    stmts: ["_", "fn g() {}"],
);

test_recover!(
    unfinished_impl_before_fn,
    "impl Foo
     fn main() {}",
    errors: ["expected token"],
    stmts: ["impl Foo {}", "fn main() {}"],
);

test_recover!(
    unfinished_pact_before_fn,
    "pact P
     fn main() {}",
    errors: ["expected token"],
    stmts: ["pact P {}", "fn main() {}"],
);

test_recover!(
    unfinished_if_before_let,
    "if x
     let y = 2;",
    errors: ["expected expression"],
    stmts: ["_", "let y = 2;"],
);

test_recover!(
    unfinished_match_arm_before_let,
    "match x {
         1 =>
     let y = 2;",
    errors: ["expected expression"],
    stmts: ["_", "let y = 2;"],
);

test_recover!(
    unclosed_group,
    "let a = (1 + 2;
     let b = 2;",
    errors: ["expected token"],
    stmts: ["let a = (1 + 2);", "let b = 2;"],
);

test_recover!(
    unclosed_square_access,
    "let a = b[1;
     let c = 3;",
    errors: ["expected token"],
    stmts: ["let a = b[1];", "let c = 3;"],
);

test_recover!(
    unclosed_array,
    "let a = [1, 2;
     let b = 2;",
    errors: ["expected token"],
    stmts: ["let a = [1, 2];", "let b = 2;"],
);

test_recover!(
    unclosed_call,
    "let a = foo(1, 2;
     let b = 2;",
    errors: ["expected token"],
    stmts: ["let a = foo(1, 2);", "let b = 2;"],
);

test_recover!(
    unclosed_call_before_exprs,
    "let x = foo(1, 2
     if x {
         bar();
     }
     let y = 3;",
    errors: ["expected token", "expected token"],
    stmts: ["let x = foo(1, 2, if x {bar()} );", "let y = 3;"],
);

test_recover!(
    unclosed_array_before_exprs,
    "let x = [1, 2
     if x {
         bar();
     }
     let y = 3;",
    errors: ["expected token", "expected token"],
    stmts: ["let x = [1, 2, if x {bar()} ];", "let y = 3;"],
);

test_recover!(
    broken_list_keeps_its_own_closer,
    "fn f() {
         match x { a => 1 b => 2 }
         foo();
     }
     let z = 1;",
    errors: ["expected token"],
    stmts: ["_", "let z = 1;"],
);

test_recover!(
    error_in_a_block_keeps_the_next_expr_stmt,
    "fn f() {
         g(1 2)
     }
     h();
     let z = 1;",
    errors: ["expected token"],
    stmts: ["fn f() {g(1, 2) }", "h()", "let z = 1;"],
);

test_recover!(
    broken_match_keeps_the_next_expr_stmt,
    "match x {
         a => 1 b => 2
     }
     foo();",
    errors: ["expected token"],
    stmts: ["_", "foo()"],
);

test_recover!(
    junk_after_a_value_ends_at_its_line,
    "let a = 1 2 3
     foo();
     let b = 2;",
    errors: ["unexpected token"],
    stmts: ["let a = 1;", "foo()", "let b = 2;"],
);

test_single_error!(
    broken_list_does_not_truncate_its_block,
    "expected token",
    "fn f() {
         let d = ~{ a = 1 b = 2 };
         foo();
     }",
    "fn f() {
         let w = Foo { a = [1 2] };
         foo();
     }",
    "fn f() {
         enum E { A B }
         g();
     }",
);

test_single_error!(
    list_missing_comma,
    "expected token",
    "foo(1 2);",
    "let a = [1 2];",
);

test_recover!(
    fstring_interp_error,
    r#"let a = f"{1 +}";
     let b = 2;"#,
    errors: ["unexpected end of input"],
    stmts: ["let a = f\"{1 + <poison>}\";", "let b = 2;"],
);

test_recover!(
    fstring_two_errors_in_one_interp,
    r#"let a = f"{(1 + ) * (2 - )}";
     let b = 2;"#,
    errors: ["expected expression", "expected expression"],
    stmts: ["let a = f\"{(1 + <poison>) * (2 - <poison>)}\";", "let b = 2;"],
);

test_recover!(
    fstring_interp_leftovers,
    r#"let a = f"{x y z}";
     let b = 2;"#,
    errors: ["unexpected token"],
    stmts: ["let a = f\"{x}\";", "let b = 2;"],
);

test_recover!(
    fstring_unterminated_interp,
    r#"let a = f"{a";
     let b = 2;"#,
    errors: ["unterminated f-string expression"],
    stmts: ["let a = f\"\";", "let b = 2;"],
);

test_single_error!(
    fstring_interp_lex_error,
    "this block comment was never closed with `*/`",
    r#"let a = f"{a /* }";"#,
);

test_single_error!(
    fstring_unterminated_interp_reports_once,
    "unterminated f-string expression",
    r#"let a = f"{a +";"#,
    r#"let a = f"{";"#,
    "f\"{".repeat(100),
);

test_recover!(
    stray_semicolon,
    "let a = 1;;
     let b = 2;",
    errors: ["unexpected token"],
    stmts: ["let a = 1;", "let b = 2;"],
);

test_recover!(
    stray_semicolon_keeps_the_next_statement,
    "let a = 1;;
     foo();
     bar();
     let c = 2;",
    errors: ["unexpected token"],
    stmts: ["let a = 1;", "foo()", "bar()", "let c = 2;"],
);

test_recover!(
    every_invalid_member_is_reported,
    "impl Foo { let a = 1; let b = 2; fn ok() {} }",
    errors: ["invalid impl item", "invalid impl item"],
    stmts: ["_"],
);

test_recover!(
    stray_closers,
    "let a = 1;
     ) ] }
     let b = 2;",
    errors: ["unexpected token"],
    stmts: ["let a = 1;", "let b = 2;"],
);

test_recover!(
    stray_closer_after_call,
    "foo(a, b))
     bar();
     baz();",
    errors: ["unexpected token"],
    stmts: ["foo(a, b)", "bar()", "baz()"],
);

test_recover!(
    stray_closer_and_semicolon,
    "let a = (1 + 2));
     let b = 2;
     c();",
    errors: ["unexpected token"],
    stmts: ["let a = (1 + 2);", "let b = 2;", "c()"],
);

test_recover!(
    stray_closer_in_block,
    "fn f() {
         foo(a));
         bar();
     }",
    errors: ["unexpected token"],
    stmts: ["fn f() {foo(a) bar()}"],
);

test_recover!(
    missing_semicolon,
    "let a = 1
     let b = 2;",
    errors: ["missing semicolon"],
    stmts: ["let a = 1;", "let b = 2;"],
);

test_recover!(
    every_line_broken,
    "let a = ;
     let b = ;
     let c = ;
     let d = 4;",
    errors: ["expected expression", "expected expression", "expected expression"],
    stmts: [
        "let a = <poison>;",
        "let b = <poison>;",
        "let c = <poison>;",
        "let d = 4;",
    ],
);

test_recover!(
    mixed_errors,
    "let a = 1
     let b = ;
     foo(a = 1, 2);
     let c = 3;",
    errors: [
        "missing semicolon",
        "expected expression",
        "named argument before positional",
    ],
    stmts: ["let a = 1;", "let b = <poison>;", "foo(a=1, 2)", "let c = 3;"],
);

test_recover!(
    invalid_assignment_target,
    "a() = 1;
     let b = 2;",
    errors: ["invalid assignment target"],
    stmts: ["<poison> = 1", "let b = 2;"],
);

test_recover!(
    keyword_as_binding,
    "let if = 1;
     let b = 2;",
    errors: ["expected pattern"],
    stmts: ["let <poison> = 1;", "let b = 2;"],
);

test_recover!(
    misdirected_and,
    "if a and b {}
     let c = 1;",
    errors: ["unknown operator `and`"],
    stmts: ["_", "let c = 1;"],
);

test_recover!(
    misdirected_as,
    "let x = a as int;
     let y = 2;",
    errors: ["mimas has no `as` casts"],
    stmts: ["_", "let y = 2;"],
);

test_recover!(
    assignment_in_condition,
    "while x = 1 {}
     let c = 1;",
    errors: ["invalid assignment in a condition"],
    stmts: ["_", "let c = 1;"],
);

test_recover!(
    error_stays_in_fn_body,
    "fn f() {
         let a = ;
         let b = 2;
     }
     let c = 3;",
    errors: ["expected expression"],
    stmts: ["fn f() {let a = <poison>; let b = 2;}", "let c = 3;"],
);

test_recover!(
    errors_in_sibling_fns,
    "fn f() {
         let a = ;
     }
     fn g() {
         let b = ;
     }
     let c = 3;",
    errors: ["expected expression", "expected expression"],
    stmts: ["fn f() {let a = <poison>;}", "fn g() {let b = <poison>;}", "let c = 3;"],
);

test_recover!(
    unclosed_fn_body,
    "fn f() {
         let a = 1;",
    errors: ["unexpected end of input"],
    stmts: ["fn f() {let a = 1;}"],
);

test_recover!(
    unclosed_nested_if,
    "if a {
         if b {
             c();
         }",
    errors: ["unexpected end of input"],
    stmts: ["if a {if b {c()} } "],
);

test_recover!(
    struct_field_missing_type,
    "struct S { a: int, b: , c: str }
     let x = 1;",
    errors: ["expected type"],
    stmts: ["struct S { a: int, b: <poison>, c: str }", "let x = 1;"],
);

test_recover!(
    struct_missing_comma,
    "struct S { a: int b: int }
     let x = 1;",
    errors: ["expected token"],
    stmts: ["struct S { a: int, b: int }", "let x = 1;"],
);

test_recover!(
    tuple_struct_empty_field,
    "struct S(int, , str);
     let x = 1;",
    errors: ["unexpected token"],
    stmts: ["struct S { 0: int, 1: str }", "let x = 1;"],
);

test_recover!(
    fn_param_missing_type,
    "fn f(a: , b: int) { a }
     let x = 1;",
    errors: ["expected type"],
    stmts: ["fn f(a: <poison>, b: int) {a }", "let x = 1;"],
);

test_recover!(
    fn_params_missing_comma,
    "fn f(a b) {}
     let x = 1;",
    errors: ["expected token"],
    stmts: ["fn f(a, b) {}", "let x = 1;"],
);

test_recover!(
    fn_missing_body,
    "fn f()
     let x = 1;",
    errors: ["missing function body"],
    stmts: ["fn f() <poison>", "let x = 1;"],
);

test_recover!(
    fn_missing_name,
    "fn (a) {}
     let x = 1;",
    errors: ["expected identifier"],
    stmts: ["_", "let x = 1;"],
);

test_recover!(
    pact_pub_marker,
    "pact P {
         pub fn a(self);
         fn b(self);
     }
     let x = 1;",
    errors: ["invalid pub marker"],
    stmts: ["pact P { fn a(self); fn b(self); }", "let x = 1;"],
);

test_recover!(
    pact_invalid_item,
    "pact P {
         let x = 1;
         fn b(self);
     }
     let y = 2;",
    errors: ["invalid pact item"],
    stmts: ["pact P { fn b(self); }", "let y = 2;"],
);

test_recover!(
    pact_const_missing_annotation,
    "pact P {
         const A;
         fn b(self);
     }
     let x = 1;",
    errors: ["annotation required"],
    stmts: ["pact P { const A: <poison>; fn b(self); }", "let x = 1;"],
);

test_recover!(
    pact_default_param,
    "pact P {
         fn a(self, b = 1);
         fn c(self);
     }
     let x = 1;",
    errors: ["default parameter value not allowed in pact signature"],
    stmts: ["pact P { fn a(self, b = 1); fn c(self); }", "let x = 1;"],
);

test_recover!(
    pact_two_default_params,
    "pact P {
         fn a(self, b = 1, c = 2);
     }
     let x = 1;",
    errors: [
        "default parameter value not allowed in pact signature",
        "default parameter value not allowed in pact signature",
    ],
    stmts: ["_", "let x = 1;"],
);

test_recover!(
    impl_invalid_item,
    "impl S {
         let x = 1;
         fn f(self) {}
     }
     let y = 2;",
    errors: ["invalid impl item"],
    stmts: ["impl S { fn f(self) {} }", "let y = 2;"],
);

test_recover!(
    impl_lone_pub,
    "impl S {
         pub
     }
     fn main() {}
     let y = 2;",
    errors: ["invalid impl item"],
    stmts: ["impl S {}", "fn main() {}", "let y = 2;"],
);

test_recover!(
    pact_lone_pub,
    "pact P {
         fn a(self);
         pub
     }
     fn main() {}
     let y = 2;",
    errors: ["invalid pub marker"],
    stmts: ["pact P { fn a(self); }", "fn main() {}", "let y = 2;"],
);

test_recover!(
    impl_missing_target,
    "impl P for {}
     let x = 1;",
    errors: ["expected identifier"],
    stmts: ["_", "let x = 1;"],
);

test_recover!(
    enum_bad_member,
    "enum E { A, 3, B }
     let x = 1;",
    errors: ["unexpected token"],
    stmts: ["enum E { A {  }, B {  } }", "let x = 1;"],
);

test_recover!(
    enum_tuple_empty_member,
    "enum E { A(int, , str), B }
     let x = 1;",
    errors: ["unexpected token"],
    stmts: ["enum E { A(int, str), B {  } }", "let x = 1;"],
);

test_recover!(
    use_missing_segment,
    "use a::;
     let x = 1;",
    errors: ["expected import"],
    stmts: ["_", "let x = 1;"],
);

test_recover!(
    use_glob_all,
    "use *;
     let x = 1;",
    errors: ["cannot glob-import all modules"],
    stmts: ["_", "let x = 1;"],
);

test_recover!(
    const_missing_value,
    "const A = ;
     let x = 1;",
    errors: ["expected expression"],
    stmts: ["const A = <poison>;", "let x = 1;"],
);

test_recover!(
    array_pattern,
    "let [a] = x;
     let b = 2;",
    errors: ["expected pattern"],
    stmts: ["let <poison> = x;", "let b = 2;"],
);

test_recover!(
    unclosed_tuple_pattern,
    "let (a, b = x;
     let c = 2;",
    errors: ["expected token"],
    stmts: ["let (a, b) = x;", "let c = 2;"],
);

test_recover!(
    match_bad_arm_pattern,
    "match x {
         + => 1,
         y => 2,
     }
     let z = 3;",
    errors: ["unexpected token"],
    stmts: ["_", "let z = 3;"],
);

test_recover!(
    match_missing_fat_arrow,
    "match x {
         1 2,
         y => 3,
     }
     let z = 3;",
    errors: ["expected token"],
    stmts: ["_", "let z = 3;"],
);

test_recover!(
    match_bad_struct_pattern,
    "match x {
         S { a = } => 1,
         y => 2,
     }
     let z = 3;",
    errors: ["expected pattern"],
    stmts: ["_", "let z = 3;"],
);

test_recover!(
    match_errors_in_two_arms,
    "match x {
         + => 1,
         y => ,
         z => 3,
     }
     let w = 4;",
    errors: ["unexpected token", "expected expression"],
    stmts: ["_", "let w = 4;"],
);

test_recover!(
    missing_annotation,
    "let a: = 1;
     let b = 2;",
    errors: ["expected type"],
    stmts: ["let a: <poison> = 1;", "let b = 2;"],
);

test_recover!(
    doubled_option_annotation,
    "let a: int?? = 1;
     let b = 2;",
    errors: ["doubled option"],
    stmts: ["let a: int? = 1;", "let b = 2;"],
);

test_recover!(
    single_type_parens,
    "fn f(a: (int)) {}
     let x = 1;",
    errors: ["parentheses around a single type do nothing"],
    stmts: ["fn f(a: int) {}", "let x = 1;"],
);

test_recover!(
    unclosed_array_annotation,
    "let a: [int = 1;
     let b = 2;",
    errors: ["expected token"],
    stmts: ["let a: [int] = 1;", "let b = 2;"],
);

test_recover!(
    unclosed_dictionary_annotation,
    "let a: ~{int = 1;
     let b = 2;",
    errors: ["expected token"],
    stmts: ["let a: ~{int} = 1;", "let b = 2;"],
);

test_single_error!(
    truncated_stmts,
    "unexpected end of input",
    "let",
    "let a",
    "let a =",
    "let a = 1 +",
    "let a: ",
    "let (a, ",
    "x = ",
    "const A: ",
    "module",
);

test_single_error!(
    truncated_exprs,
    "unexpected end of input",
    "foo(",
    "f(a = ",
    "a.",
    "|a| ",
    "S { a = ",
    "if true",
    "if a {} else",
    "while",
    "for x in",
    "match x {",
    "match x { 1 =>",
);

test_single_error!(
    truncated_nested,
    "unexpected end of input",
    "fn f() { fn g() {",
    "fn f() { if a { loop {",
    "impl S { fn f() { let a = (",
);

test_single_error!(
    truncated_delimiters,
    "unexpected end of input",
    "{",
    "[",
    "(",
    "~{",
);

test_single_error!(
    truncated_items,
    "unexpected end of input",
    "fn f(",
    "fn f() {",
    "struct S {",
    "enum E {",
    "impl S {",
    "pact P {",
    "use a::",
);

test_survives!(
    deep_nesting,
    "(".repeat(10_000),
    "[".repeat(10_000),
    "{".repeat(10_000),
    "-".repeat(10_000),
    "|| ".repeat(10_000),
    "let a: [".repeat(10_000),
    "match x { (".repeat(10_000),
    "loop ".repeat(10_000),
    "while a ".repeat(10_000),
    "for a in b ".repeat(10_000),
    "if a {} else ".repeat(10_000),
    "f\"{".repeat(10_000),
    "fn f() { ".repeat(10_000),
    "impl A { fn f() { ".repeat(10_000),
    "pact P { fn f() { ".repeat(10_000),
);

test_single_error!(
    nesting_too_deep_reports_once,
    "nesting too deep",
    "-".repeat(10_000),
    "- ".repeat(10_000),
    "a + (".repeat(10_000),
);

test_survives!(
    unbalanced_closers,
    ")",
    "]",
    "}",
    ") ] } ) ] }",
    "}}}}}}}}",
    "fn f() {}}
     let x = 1;",
    "let a = (1 + 2));
     let b = 2;",
);

test_survives!(
    token_soup,
    "let fn struct = => :: ;; }{ )(",
    "match match match",
    "fn fn fn",
    "impl impl impl",
    "pub pub pub",
    "else else else",
    "= = = = =",
    ", , , , ,",
    ":: :: ::",
    "?? ?? ??",
    "let let let let;",
    "struct { } enum { } pact { } impl { }",
);

test_survives!(
    lexer_errors,
    "let a = \"abc",
    r#"let a = "abc
     let b = 2;"#,
    "let a = f\"abc",
    "let a = f\"{\"abc}\";",
    "/* never closed",
    "let a = 1; /* never closed",
    "let a = 0x;",
);

test_error_label!(
    fstring_interp_error_points_into_the_source,
    r#"let a = 1;
let b = f"é{c} and {(1 + ) * 2}";"#,
    ")",
);

test_error_label!(
    unterminated_block_comment_has_a_span,
    "let a = 1; /* abc",
    "/*",
);

test_recover!(
    errors_come_out_in_source_order,
    "let a = ;
     let b = 0x;
     let c = ;",
    errors: [
        "expected expression",
        "this is not a valid hexidecimal value",
        "expected expression",
    ],
    stmts: ["let a = <poison>;", "_", "let c = <poison>;"],
);

#[test]
fn lone_expr_reports_its_poison() {
    let mut parser = Parser::new(Lexer::new("1 + ", 0, "test".into()));
    let expr = parser.expr();
    assert_eq!(expr.to_string(), "1 + <poison>");
    assert_eq!(parser.errors().len(), 1);
}

#[test]
fn every_prefix_of_sample_terminates() {
    assert!(
        recover(SAMPLE).1.is_empty(),
        "the sample should parse cleanly"
    );
    for (i, _) in SAMPLE.char_indices() {
        recover(&SAMPLE[..i]);
    }
}

#[test]
fn every_char_deletion_of_sample_terminates() {
    for (i, c) in SAMPLE.char_indices() {
        let mut source = SAMPLE.to_owned();
        source.replace_range(i..i + c.len_utf8(), "");
        recover(&source);
    }
}

#[test]
fn every_stray_token_insertion_in_sample_terminates() {
    for (i, _) in SAMPLE.char_indices() {
        for stray in [
            ")", "]", "}", "(", "[", "{", ";", ",", "=", "::", "|", "fn", "let",
        ] {
            let mut source = SAMPLE.to_owned();
            source.insert_str(i, stray);
            recover(&source);
        }
    }
}

const SAMPLE: &str = r#"module demo;

use std::math::{abs, max};

const LIMIT: int = 10;

pub struct Point {
    pub x: int,
    y: int,
}

enum Shape {
    Circle(float),
    Rect { w: int, h: int },
}

pact Area {
    fn area(self) -> float;
}

impl Area for Shape {
    fn area(self) -> float {
        match self {
            Shape::Circle(r) => r * r * 3.14,
            Shape::Rect { w, h } => (w * h).to_float(),
        }
    }
}

fn sum(values: [int], start: int = 0) -> int? {
    let total = start;
    for v in values {
        if v > LIMIT {
            continue;
        }
        total += v;
    }
    total
}

let p = Point { x = 1, y = -2 };
let names = ~{ a = "x", b = f"{p.x}!" };
let f = |a: int, b| a + b;
let pair = (sum([1, 2, 3], start = 4), names["a"]?);
while let (a, b) = pair {
    break;
}
"#;

fn check_recovery(source: &str, expected_errors: &[&str], expected_stmts: &[&str]) {
    let (stmts, errors) = recover(source);
    pretty_assertions::assert_eq!(errors, expected_errors, "errors for `{source}`");
    assert_eq!(
        stmts.len(),
        expected_stmts.len(),
        "stmts for `{source}`: {stmts:#?}"
    );
    for (stmt, expected) in stmts.iter().zip(expected_stmts) {
        if *expected != "_" {
            pretty_assertions::assert_eq!(stmt, expected, "stmts for `{source}`");
        }
    }
}

/// Parses `source` into its stmts and errors, and checks that poison only ever shows up
/// alongside an error.
fn recover(source: &str) -> (Vec<String>, Vec<String>) {
    let (stmts, errors) = parse_to_strings(source);
    let poisoned = stmts.iter().any(|stmt| stmt.contains(POISON));
    assert!(
        !poisoned || !errors.is_empty(),
        "`{}` produced poison without an error",
        preview(source)
    );
    (stmts, errors)
}

fn preview(source: &str) -> String {
    const MAX: usize = 80;
    if source.chars().count() <= MAX {
        source.to_owned()
    } else {
        format!("{}...", source.chars().take(MAX).collect::<String>())
    }
}
