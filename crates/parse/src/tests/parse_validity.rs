// assignment targets
test_ok!(
    valid_assignment_targets,
    "a = 1;",
    "a.b = 1;",
    "a[0] = 1;",
    "a.b.c = 1;"
);
test_fail!(
    invalid_assignment_targets,
    "a() = 1;",
    "fn() {} = 1;",
    "(a) = 1;",
    "true = 1;",
    "1 = 1;",
    "a + b = 1;"
);

// dot access
test_ok!(valid_dot_access, "a = b.c;", "a = b.0;");
test_fail!(invalid_dot_access_operator, "a = b.+;", "a = b.;");

// call arguments
test_ok!(
    positional_then_named_ok,
    "foo(1, a=2);",
    "foo(1, 2, a=3, b=4);"
);
test_fail!(named_before_positional, "foo(a=1, 2);", "foo(x=1, y=2, 3);");

// missing semicolons / unexpected tokens
test_fail!(missing_semicolon_let, "let a = 5", "let a = 5 let b = 6;");
test_fail!(missing_semicolon_assign, "a = 1");
test_fail!(unexpected_token_close_paren, "a = );", "a = ;");
test_fail!(unexpected_token_double_colon_leading, "a = ::;");

// expected identifier
test_fail!(
    expected_identifier_stray_symbol,
    "a = @;",
    "a = `;",
    "a = #;"
);
test_fail!(
    keyword_as_name,
    "let if = 1;",
    "let fn = 1;",
    "let self = 1;",
    "let match = 1;",
    "let return = 1;"
);

// unexpected end of input
test_fail!(unexpected_end_let, "let", "let a", "let a =", "let a = 1 +");
test_fail!(unexpected_end_block, "fn f() {", "if true", "match x {");

// unmatched brackets
test_fail!(unmatched_brackets, "{", "[", "(", "~{", "fn foo() {");

// impl items
test_ok!(
    valid_impl_items,
    "impl Foo { fn x() {} }",
    "impl Foo { const X: int = 1; }",
    "impl Drawable for Foo {}",
    "impl Drawable for Foo { fn draw() {} }"
);
test_fail!(
    invalid_impl_item,
    "impl Foo { let x = 1; }",
    "impl Foo { struct S; }",
    "impl Foo for Bar { x = 1; }"
);

// pact items
test_ok!(
    valid_pact_items,
    "pact Foo { fn bar() -> (); }",
    "pact Foo { const BAR: int; }",
    "pact Foo { fn bar() { 0 } }"
);
test_fail!(
    invalid_pact_item,
    "pact Foo { let x = 1; }",
    "pact Foo { struct S; }"
);
test_fail!(pact_const_annotation_required, "pact Foo { const BAR; }");
test_fail!(
    pact_const_with_default_rejected,
    "pact Foo { const BAR: int = 0; }"
);
test_fail!(
    pact_sig_default_param_rejected,
    "pact Foo { fn bar(x: int = 0); }"
);
test_fail!(
    pact_pub_marker_rejected,
    "pact Foo { pub fn bar() -> (); }",
    "pact Foo { pub const BAR: int; }"
);
test_fail!(
    pact_sig_missing_semi,
    "pact Foo { fn bar() -> () }",
    "pact Foo { const BAR: int }"
);

// fn body required at top level
test_ok!(fn_with_body_ok, "fn foo() {}", "fn foo() -> int { 0 }");
test_fail!(missing_fn_body_toplevel, "fn foo();", "fn foo() -> int;");

// modules
test_ok!(single_module_ok, "module a;");
test_ok!(nested_module_path_ok, "module a::b;");

// a second `module` decl is caught by `module_name()`, not by `into_ast`
#[test]
fn already_inside_module() {
    let lexer = crate::lex::Lexer::new("module a; module b;", 0, "test".into());
    let ast = crate::Parser::new(lexer).into_ast().unwrap();
    assert!(
        ast.module_name().is_err(),
        "second `module` decl should be rejected"
    );
}

#[test]
fn single_module_name_ok() {
    let lexer = crate::lex::Lexer::new("module a;", 0, "test".into());
    let ast = crate::Parser::new(lexer).into_ast().unwrap();
    assert!(ast.module_name().is_ok());
}

// use
test_ok!(
    ordinary_use_ok,
    "use a::b;",
    "use foo::{ bar, baz };",
    "use foo::{ bar, baz, };",
    "use foo::*;"
);
test_fail!(glob_all_modules_rejected, "use a::*::*;");

// type annotations
test_ok!(tuple_type_one_element_ok, "let i: (int,) = z;");
test_fail!(
    single_type_parens_rejected,
    "fn f(x: (int)) -> int { 0 }",
    "let i: (int) = z;"
);
test_ok!(
    single_optional_annotation_ok,
    "let x: int? = a;",
    "fn f(x: int?) -> int { 0 }"
);
test_fail!(
    double_hook_annotation_rejected,
    "fn f(x: int??) -> int { 0 }",
    "let x: int?? = a;"
);

// pact bounds in annotations
test_ok!(
    pact_bound_parenthesized_ok,
    "fn f(d: (Foo + Bar)?) -> int { 0 }",
    "fn f(d: Foo + Bar) -> int { 0 }",
    "fn f(d: Foo + Bar + Baz) -> int { 0 }"
);
test_fail!(
    pact_bound_constraint_rejected,
    "fn f(d: Foo + Bar?) -> int { 0 }",
    "fn f(d: Foo? + Bar) -> int { 0 }",
    "fn f(d: Foo! + Bar?) -> int { 0 }"
);
test_fail!(
    non_pact_in_bound_rejected,
    "fn f(d: int + Foo) -> int { 0 }",
    "fn f(d: Foo + str) -> int { 0 }"
);

// reserved-but-unparsed syntax
test_fail!(let_mut_not_parsed, "let mut x = 5;");
test_fail!(
    as_cast_not_parsed,
    "a = 5 as float;",
    "let x: float = 5 as float;"
);
test_fail!(
    word_operators_not_parsed,
    "a = true and false;",
    "a = true or false;",
    "a = not true;"
);
test_fail!(
    elif_not_parsed,
    "a = if true { 1 } elif false { 2 } else { 3 };"
);
test_fail!(single_quoted_string_rejected, "a = 'foo';");

// `1.2.3` lexes as Float(1.2) then `.3` tuple-index; parser accepts it
// (a solver error, NotATuple, catches it later) so it PARSES
test_ok!(malformed_number_parses_as_tuple_index, "a = 1.2.3;");

// access target permissiveness — nonsensical receivers parse fine; the solver lints, not the parser
test_ok!(
    access_targets,
    "a = b.c.d;",
    "a = b().c;",
    "a = (b).c;",
    "a = 0.a;",
    "a = 0.0.a;",
    r#"a = "foo".a;"#,
    "a = [].a;",
    "a = ~{}.a;",
    "a = 0();",
    "a = 0[0];",
    r#"a = 0["foo"];"#,
    "a = 0.0[0];",
    r#"a = 0.0["foo"];"#,
    "a = [][0];",
    r#"a = []["foo"];"#,
    "a = ~{}[0];",
    r#"a = ~{}["foo"];"#,
    "a = true();",
    "a = true[0];",
    r#"a = true["foo"];"#,
    "a = \"foo\"[0];",
    r#"a = "foo"["foo"];"#,
    r#"a = "hello".len();"#
);
test_ok!(
    call_targets,
    "a.b();",
    "a()();",
    "(a)();",
    "true();",
    "0();",
    "0.0();",
    "\"foo\"();",
    "~{}();",
    "[]();"
);
test_ok!(
    equality_targets,
    "a = b.c == c;",
    "a = b() == c;",
    "a = (b) == c;",
    "a = true == b;"
);
test_ok!(
    evaluation_targets,
    "a = b.c + c;",
    "a = b() + c;",
    "a = (b) + c;",
    "a = true + b;",
    "a = b++ + b;"
);
test_ok!(
    logical_targets,
    "a = b.c && c;",
    "a = b() && c;",
    "a = (b) && c;",
    "a = true && b;"
);
test_ok!(
    unwrap_targets,
    "a = b!;",
    "a = b.c!;",
    "a = b()!;",
    "a = b!.c;",
    "a = b!()();"
);
test_ok!(
    coalescence_targets,
    "a = b ?? c;",
    "a = b.c ?? d;",
    "a = b() ?? c;",
    "a = (b) ?? c;"
);
test_ok!(
    in_targets,
    "a = 0 in b;",
    "a = b in c;",
    r#"a = "x" in "xyz";"#
);

// misc valid forms
test_ok!(while_statement_no_semicolon, "while true {}");
test_ok!(empty_tuple_struct, "struct Foo();");
test_ok!(struct_literal_in_grouping, "a = (Foo {});");
test_ok!(
    struct_literal_in_if_body,
    "a = if true { Foo {} } else { Foo {} };"
);
test_ok!(pattern_match_or, "match x { 0 | 1 | 2 => 1, _ => 2 }");
test_ok!(match_arm_with_guard, "match x { 0 if true => 1, _ => 2 }");
test_ok!(struct_literal_trailing_comma, "a = Foo { a = 1, };");
test_ok!(
    fstring_valid,
    r#"a = f"hello";"#,
    r#"a = f"hello {name}";"#,
    r#"a = f"{1 + 2}";"#
);
test_ok!(closure_with_return_type, "a = |a: int| -> int { a };");
test_ok!(
    pact_default_body_no_semi,
    "pact Foo { fn bar() { 0 } }",
    "pact Foo { fn a() { 0 } fn b(); fn c() -> int { 1 } }"
);

#[test]
fn deep_nesting_does_not_overflow_the_stack() {
    let src: &'static str = Box::leak(
        format!(
            "fn main() {{ let x = {}1{}; }}",
            "(".repeat(4000),
            ")".repeat(4000)
        )
        .into_boxed_str(),
    );
    crate::tests::utils::assert_rejects(src);
}

// `T??` lexes as DoubleHook; the annotation loop turns it into a targeted diagnostic
test_fail!(doubled_option_annotation, "let a: int?? = 5;");

// misdirection: common habits from other languages get a teaching error
test_fail!(misdirect_elif, "if x == 1 {} elif x == 2 {}");
test_fail!(misdirect_and, "if a and b {}");
test_fail!(misdirect_or, "let c = a or b;");
test_fail!(misdirect_as, "let y = x as float;");
test_fail!(misdirect_if_assign, "if x = 5 {}");

#[test]
fn misdirection_renders_spelling() {
    let lexer = crate::lex::Lexer::new("if a and b {}", 0, "test".into());
    let err = crate::Parser::new(lexer).into_ast().unwrap_err();
    let rendered = format!("{err:?}");
    assert!(
        rendered.contains("&&"),
        "the `and` hint should teach `&&`: {rendered}"
    );
}
