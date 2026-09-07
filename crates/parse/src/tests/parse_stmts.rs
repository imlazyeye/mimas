use crate::{
    IntoExpr, IntoStmt, block, components::*, expr::*, float, ident, ident_expr, int, item::*,
    lex::TyKw, stmt::*,
};
#[cfg(test)]
use crate::{enu, enum_struct, enum_tuple, struc};
use pretty_assertions::assert_eq;

macro_rules! stmt_test {
    ($name:ident, $source:expr, $expected:expr) => {
        #[cfg(test)]
        #[test]
        fn $name() {
            let kind: crate::StmtKind = $expected.into();
            let expected = kind.into_stmt();
            let lexer = crate::lex::Lexer::new($source, 0, "test".into());
            let mut parser = crate::Parser::new(lexer);
            let output = parser.stmt().unwrap();
            assert_eq!(output.kind(), expected.kind(), "`{}` failed!", $source);
        }
    };
}

stmt_test!(
    let_inferred,
    "let a = 0;",
    Let::new(ident!("a").into(), int!(0), None)
);

stmt_test!(
    let_explicate_bool,
    "let a: bool = true;",
    Let::new(
        ident!("a").into(),
        Literal::True.into_expr(),
        Some(Annotation::Kw(TyKw::Bool)),
    )
);
stmt_test!(
    let_explicate_int,
    "let a: int = 1;",
    Let::new(ident!("a").into(), int!(1), Some(Annotation::Kw(TyKw::Int)))
);
stmt_test!(
    let_explicate_float,
    "let a: float = 0.0;",
    Let::new(
        ident!("a").into(),
        float!(0.0),
        Some(Annotation::Kw(TyKw::Float)),
    )
);
stmt_test!(
    let_explicate_hex,
    "let a: int = 0xffffff;",
    Let::new(
        ident!("a").into(),
        Literal::Hex("ffffff".into()).into_expr(),
        Some(Annotation::Kw(TyKw::Int)),
    )
);
stmt_test!(
    let_explicate_string,
    r#"let a: str = "foo";"#,
    Let::new(
        ident!("a").into(),
        Literal::String("foo".into()).into_expr(),
        Some(Annotation::Kw(TyKw::Str)),
    )
);
stmt_test!(
    let_explicate_array,
    "let a: [int] = [0];",
    Let::new(
        ident!("a").into(),
        Literal::Array(vec![int!(0)]).into_expr(),
        Some(Annotation::Array(Box::new(Annotation::Kw(TyKw::Int)))),
    )
);
stmt_test!(
    let_explicate_dictionary,
    "let a: ~{int} = ~{ a = 0 };",
    Let::new(
        ident!("a").into(),
        Literal::Dictionary(vec![(ident!("a"), int!(0))]).into_expr(),
        Some(Annotation::Dictionary(Box::new(Annotation::Kw(TyKw::Int)))),
    )
);
stmt_test!(
    let_type_obj,
    "let a: Foo = Foo {};",
    Let::new(
        ident!("a").into(),
        Literal::Struct(StructLiteral {
            name: ident_expr!("Foo"),
            fields: vec![]
        })
        .into_expr(),
        Some(Annotation::Ty(ident!("Foo"))),
    )
);

stmt_test!(
    let_fn_annotation,
    "let f: (int) -> int = 0;",
    Let::new(
        ident!("f").into(),
        int!(0),
        Some(Annotation::Function(
            vec![Annotation::Kw(TyKw::Int)],
            Box::new(Annotation::Kw(TyKw::Int)),
        )),
    )
);
stmt_test!(
    let_fn_annotation_no_params,
    "let f: () -> str = 0;",
    Let::new(
        ident!("f").into(),
        int!(0),
        Some(Annotation::Function(
            vec![],
            Box::new(Annotation::Kw(TyKw::Str)),
        )),
    )
);
stmt_test!(
    let_fn_annotation_multi_params,
    "let f: (int, str) -> bool = 0;",
    Let::new(
        ident!("f").into(),
        int!(0),
        Some(Annotation::Function(
            vec![Annotation::Kw(TyKw::Int), Annotation::Kw(TyKw::Str)],
            Box::new(Annotation::Kw(TyKw::Bool)),
        )),
    )
);
stmt_test!(
    let_fn_annotation_higher_order,
    "let f: (int) -> (int) -> int = 0;",
    Let::new(
        ident!("f").into(),
        int!(0),
        Some(Annotation::Function(
            vec![Annotation::Kw(TyKw::Int)],
            Box::new(Annotation::Function(
                vec![Annotation::Kw(TyKw::Int)],
                Box::new(Annotation::Kw(TyKw::Int)),
            )),
        )),
    )
);

stmt_test!(
    let_else,
    "let a = b else return;",
    Let::new_with_else(
        ident!("a").into(),
        ident_expr!("b"),
        None,
        Some(Return::new(None).into_expr())
    )
);

stmt_test!(
    let_else_null_bind,
    "let a? = b else return;",
    Let::new_with_else(
        Pat::new(
            PatKind::NullBind(Box::new(ident!("a").into())),
            shared::Location::default(),
        ),
        ident_expr!("b"),
        None,
        Some(Return::new(None).into_expr())
    )
);

stmt_test!(
    let_else_break,
    "let a = b else break;",
    Let::new_with_else(
        ident!("a").into(),
        ident_expr!("b"),
        None,
        Some(Break::new(None).into_expr())
    )
);

stmt_test!(
    cosnt_inferred,
    "const A = 0;",
    Const::new(ident!("A"), int!(0), None).into_item()
);

stmt_test!(
    cosnt_explicate,
    "const A: int = 0;",
    Const::new(ident!("A"), int!(0), Some(Annotation::Kw(TyKw::Int))).into_item()
);

stmt_test!(
    assign_block,
    "let i = { 0 };",
    Let::new(
        ident!("i").into(),
        Block::new_with_yield(vec![], int!(0)).into_expr(),
        None,
    )
);

stmt_test!(
    assign_loop,
    "let i = loop { break 0; };",
    Let::new(
        ident!("i").into(),
        Loop::new(block!(Break::new(Some(int!(0))).into_expr().into_stmt())).into_expr(),
        None,
    )
);

stmt_test!(
    assign_if,
    "let i = if a 0 else 1;",
    Let::new(
        ident!("i").into(),
        If::new_with_else(ident_expr!("a"), int!(0), int!(1)).into_expr(),
        None,
    )
);

stmt_test!(
    assign_match,
    "let i = match a { 0 => 1 };",
    Let::new(
        ident!("i").into(),
        Match::new(
            ident_expr!("a"),
            vec![MatchCase::new(
                crate::components::Pat::new(
                    crate::components::PatKind::Literal(crate::expr::Literal::Int(0)),
                    shared::Location::default()
                ),
                None,
                int!(1)
            )],
            false,
        )
        .into_expr(),
        None,
    )
);

stmt_test!(
    let_int,
    "let i: int = 0;",
    Let::new(ident!("i").into(), int!(0), Some(Annotation::Kw(TyKw::Int)))
);

stmt_test!(
    let_float,
    "let i: float = 0;",
    Let::new(
        ident!("i").into(),
        int!(0),
        Some(Annotation::Kw(TyKw::Float)),
    )
);

stmt_test!(
    let_str,
    "let i: str = 0;",
    Let::new(ident!("i").into(), int!(0), Some(Annotation::Kw(TyKw::Str)))
);

stmt_test!(
    let_array,
    "let i: [int] = 0;",
    Let::new(
        ident!("i").into(),
        int!(0),
        Some(Annotation::Array(Box::new(Annotation::Kw(TyKw::Int)))),
    )
);

stmt_test!(
    let_dictionary,
    "let i: ~{str} = 0;",
    Let::new(
        ident!("i").into(),
        int!(0),
        Some(Annotation::Dictionary(Box::new(Annotation::Kw(TyKw::Str)))),
    )
);

stmt_test!(
    let_fn,
    "let i: (int, str) -> int = 0;",
    Let::new(
        ident!("i").into(),
        int!(0),
        Some(Annotation::Function(
            vec![Annotation::Kw(TyKw::Int), Annotation::Kw(TyKw::Str)],
            Box::new(Annotation::Kw(TyKw::Int))
        )),
    )
);

stmt_test!(
    let_option,
    "let i: int? = 0;",
    Let::new(
        ident!("i").into(),
        int!(0),
        Some(Annotation::Option(Box::new(Annotation::Kw(TyKw::Int)))),
    )
);

stmt_test!(
    let_result,
    "let i: int! = 0;",
    Let::new(
        ident!("i").into(),
        int!(0),
        Some(Annotation::Result(Box::new(Annotation::Kw(TyKw::Int)))),
    )
);

stmt_test!(
    let_option_of_result,
    "let i: int!? = 0;",
    Let::new(
        ident!("i").into(),
        int!(0),
        Some(Annotation::Option(Box::new(Annotation::Result(Box::new(
            Annotation::Kw(TyKw::Int)
        ))))),
    )
);

stmt_test!(
    let_null,
    "let i = null;",
    Let::new(ident!("i").into(), Literal::Null.into_expr(), None)
);

stmt_test!(
    let_absurd_type,
    "let i: [ ~{ (int?, str, int) -> [ [ [int] ] ]? } ] = 0;",
    Let::new(
        ident!("i").into(),
        int!(0),
        Some(Annotation::Array(Box::new(Annotation::Dictionary(
            Box::new(Annotation::Function(
                vec![
                    Annotation::Option(Box::new(Annotation::Kw(TyKw::Int))),
                    Annotation::Kw(TyKw::Str),
                    Annotation::Kw(TyKw::Int),
                ],
                Box::new(Annotation::Option(Box::new(Annotation::Array(Box::new(
                    Annotation::Array(Box::new(Annotation::Array(Box::new(Annotation::Kw(
                        TyKw::Int
                    )))))
                )))))
            ))
        )))),
    )
);

stmt_test!(
    return_stmt_no_value,
    "return;",
    Return::new(None).into_expr()
);

stmt_test!(
    return_stmt_with_value,
    "return 0;",
    Return::new(Some(int!(0))).into_expr()
);

stmt_test!(break_stmt_no_value, "break;", Break::new(None).into_expr());

stmt_test!(
    break_stmt_with_value,
    "break 0;",
    Break::new(Some(int!(0))).into_expr()
);

stmt_test!(
    collect_stmt,
    "collect 0;",
    Collect::new(int!(0)).into_expr()
);

stmt_test!(
    r#use,
    "use foo;",
    Use::Singular(vec![], ident!("foo")).into_item()
);

stmt_test!(
    use_nested,
    "use foo::bar;",
    Use::Singular(vec![ident!("foo")], ident!("bar")).into_item()
);

stmt_test!(
    use_all,
    "use foo::*;",
    Use::All(vec![ident!("foo"),]).into_item()
);

stmt_test!(
    use_multi,
    "use foo::{ bar, baz };",
    Use::Multi(vec![ident!("foo")], vec![ident!("bar"), ident!("baz")]).into_item()
);

stmt_test!(
    module,
    "module foo;",
    StmtKind::Module(Module::new(ident!("foo")))
);

stmt_test!(
    identity_module,
    "module @;",
    StmtKind::Module(Module::new(ident!("test")))
);

stmt_test!(
    assign,
    "foo = 1;",
    Assignment::new(ident_expr!("foo"), AssignmentOp::Identity, int!(1),)
);

stmt_test!(
    logical_assignment,
    "foo = 1 && 1;",
    Assignment::new(
        ident_expr!("foo"),
        AssignmentOp::Identity,
        Logical::new(int!(1), LogicalOp::And, int!(1)).into_expr(),
    )
);

stmt_test!(
    null_coalecence_assign,
    "foo = bar ?? 0;",
    Assignment::new(
        ident_expr!("foo"),
        AssignmentOp::Identity,
        Coalescence::new(ident_expr!("bar"), int!(0)).into_expr(),
    )
);

// stmt_test!(
//     self_dot_assign,
//     "self.foo = 1;",
//     Assignment::new(
//         Access::Identity {
//             right: ident!("foo")
//         }
//         .into_expr(),
//         AssignmentOp::Identity,
//         int!(1),
//     )
// );

stmt_test!(
    dot_assign,
    "foo.bar = 1;",
    Assignment::new(
        Access::Dot {
            left: ident_expr!("foo"),
            right: ident_expr!("bar"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        AssignmentOp::Identity,
        int!(1),
    )
);

stmt_test!(
    array_assign,
    "foo[0] = 1;",
    Assignment::new(
        Access::Square {
            left: ident_expr!("foo"),
            key: int!(0),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        AssignmentOp::Identity,
        int!(1),
    )
);

stmt_test!(
    plus_equal,
    "foo += 1;",
    Assignment::new(ident_expr!("foo"), AssignmentOp::PlusEqual, int!(1),)
);

stmt_test!(
    minus_equal,
    "foo -= 1;",
    Assignment::new(ident_expr!("foo"), AssignmentOp::MinusEqual, int!(1),)
);

stmt_test!(
    star_equal,
    "foo *= 1;",
    Assignment::new(ident_expr!("foo"), AssignmentOp::StarEqual, int!(1),)
);

stmt_test!(
    slash_equal,
    "foo /= 1;",
    Assignment::new(ident_expr!("foo"), AssignmentOp::SlashEqual, int!(1),)
);

stmt_test!(
    and_equal,
    "foo &= 1;",
    Assignment::new(ident_expr!("foo"), AssignmentOp::AndEqual, int!(1),)
);

stmt_test!(
    or_equal,
    "foo |= 1;",
    Assignment::new(ident_expr!("foo"), AssignmentOp::OrEqual, int!(1),)
);

stmt_test!(
    xor_equal,
    "foo ^= 1;",
    Assignment::new(ident_expr!("foo"), AssignmentOp::XorEqual, int!(1),)
);

stmt_test!(
    mod_equal,
    "foo %= 1;",
    Assignment::new(ident_expr!("foo"), AssignmentOp::ModEqual, int!(1),)
);

stmt_test!(
    div_equal,
    "foo ~/= 1;",
    Assignment::new(ident_expr!("foo"), AssignmentOp::DivEqual, int!(1),)
);

stmt_test!(
    general_self_reference,
    "foo = self;",
    Assignment::new(
        ident_expr!("foo"),
        AssignmentOp::Identity,
        ident_expr!("self"),
    )
);

stmt_test!(
    comment_above_statement,
    "
            // nothing in here!
            foo = bar;    
        ",
    Assignment::new(
        ident_expr!("foo"),
        AssignmentOp::Identity,
        ident_expr!("bar"),
    )
);

stmt_test!(
    optional_semicolon_fn,
    "fn foo() {}",
    Function::new(ident!("foo"), vec![], None, block!()).into_item()
);

stmt_test!(
    optional_semicolon_if,
    "if true {}",
    If::new(Literal::True.into_expr(), block!()).into_expr()
);

stmt_test!(optional_semicolon_block, "{}", block!());

stmt_test!(
    optional_semicolon_struct,
    "struct Foo {}",
    Struct::new(ident!("Foo"), vec![]).into_item()
);

stmt_test!(
    optional_semicolon_impl,
    "impl Foo {}",
    Impl::new(ident!("Foo"), None, vec![]).into_item()
);

stmt_test!(
    impl_block,
    "impl Foo { fn foo() {} }",
    Impl::new(
        ident!("Foo"),
        None,
        vec![Function::new(ident!("foo"), vec![], None, block!()).into_item()],
    )
    .into_item()
);

stmt_test!(
    impl_pact_for_target,
    "impl Drawable for Foo {}",
    Impl::new(ident!("Foo"), Some(ident!("Drawable")), vec![]).into_item()
);

stmt_test!(
    impl_pact_with_method,
    "impl Drawable for Foo { fn draw() {} }",
    Impl::new(
        ident!("Foo"),
        Some(ident!("Drawable")),
        vec![Function::new(ident!("draw"), vec![], None, block!()).into_item()],
    )
    .into_item()
);

stmt_test!(
    empty_pact,
    "pact Foo {}",
    Pact::new(ident!("Foo"), vec![]).into_item()
);

stmt_test!(
    pact_fn,
    "pact Foo { fn foo(); }",
    Pact::new(
        ident!("Foo"),
        vec![PactItem::Fn {
            name: ident!("foo"),
            parameters: vec![],
            return_type: None,
            default: None,
        }]
    )
    .into_item()
);

stmt_test!(
    pact_fn_annotated,
    "pact Foo { fn foo() -> (); }",
    Pact::new(
        ident!("Foo"),
        vec![PactItem::Fn {
            name: ident!("foo"),
            parameters: vec![],
            return_type: Some(Annotation::Unit),
            default: None,
        }]
    )
    .into_item()
);

stmt_test!(
    pact_fn_default_body,
    "pact Foo { fn foo() { 0 } }",
    Pact::new(
        ident!("Foo"),
        vec![PactItem::Fn {
            name: ident!("foo"),
            parameters: vec![],
            return_type: None,
            default: Some(Block::new_with_yield(vec![], int!(0)).into_expr()),
        }]
    )
    .into_item()
);

stmt_test!(
    pact_const,
    "pact Foo { const FOO: int; }",
    Pact::new(
        ident!("Foo"),
        vec![PactItem::Const {
            name: ident!("FOO"),
            annotation: Annotation::Kw(TyKw::Int)
        }]
    )
    .into_item()
);

stmt_test!(
    pact_multi_member,
    "pact Foo { const ID: int; fn name() -> str; fn cost() -> int; }",
    Pact::new(
        ident!("Foo"),
        vec![
            PactItem::Const {
                name: ident!("ID"),
                annotation: Annotation::Kw(TyKw::Int),
            },
            PactItem::Fn {
                name: ident!("name"),
                parameters: vec![],
                return_type: Some(Annotation::Kw(TyKw::Str)),
                default: None,
            },
            PactItem::Fn {
                name: ident!("cost"),
                parameters: vec![],
                return_type: Some(Annotation::Kw(TyKw::Int)),
                default: None,
            },
        ]
    )
    .into_item()
);

stmt_test!(
    optional_semicolon_loop,
    "loop {}",
    Loop::new(block!()).into_expr()
);

stmt_test!(
    optional_semicolon_for,
    "for a in b {}",
    For::new(ident!("a"), ident_expr!("b"), block!()).into_expr()
);

stmt_test!(
    optional_semicolon_match,
    "match foo {}",
    Match::new(ident_expr!("foo"), vec![], false).into_expr()
);

stmt_test!(
    deconstruct_tuple,
    "let (a, b) = c;",
    Let::new(
        Pat::new(
            PatKind::Tuple(vec![
                Pat::new(ident!("a").into(), shared::Location::default()),
                Pat::new(ident!("b").into(), shared::Location::default()),
            ]),
            shared::Location::default()
        ),
        ident!("c").into_expr(),
        None,
    )
);

stmt_test!(
    let_option_array,
    "let i: [int]? = null;",
    Let::new(
        ident!("i").into(),
        Literal::Null.into_expr(),
        Some(Annotation::Option(Box::new(Annotation::Array(Box::new(
            Annotation::Kw(TyKw::Int)
        ))))),
    )
);

stmt_test!(
    let_tuple_explicit,
    "let i: (int, str) = (0, \"a\");",
    Let::new(
        ident!("i").into(),
        Literal::Tuple(vec![int!(0), Literal::String("a".into()).into_expr()]).into_expr(),
        Some(Annotation::Tuple(vec![
            Annotation::Kw(TyKw::Int),
            Annotation::Kw(TyKw::Str)
        ])),
    )
);

stmt_test!(
    let_one_tuple_explicit,
    "let i: (int,) = z;",
    Let::new(
        ident!("i").into(),
        ident_expr!("z"),
        Some(Annotation::Tuple(vec![Annotation::Kw(TyKw::Int)])),
    )
);

stmt_test!(
    let_pact_bound,
    "let i: Foo + Bar = z;",
    Let::new(
        ident!("i").into(),
        ident_expr!("z"),
        Some(Annotation::Bounds(vec![ident!("Foo"), ident!("Bar")])),
    )
);

stmt_test!(
    let_pact_bound_grouped_optional,
    "let i: (Foo + Bar)? = z;",
    Let::new(
        ident!("i").into(),
        ident_expr!("z"),
        Some(Annotation::Option(Box::new(Annotation::Bounds(vec![
            ident!("Foo"),
            ident!("Bar")
        ])))),
    )
);

stmt_test!(
    plus_equal_on_dot_access,
    "foo.bar += 1;",
    Assignment::new(
        Access::Dot {
            left: ident_expr!("foo"),
            right: ident_expr!("bar"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        AssignmentOp::PlusEqual,
        int!(1),
    )
);

stmt_test!(
    const_negative_int,
    "const A = -1;",
    Const::new(
        ident!("A"),
        Unary::new(UnaryOp::Negative, int!(1)).into_expr(),
        None,
    )
    .into_item()
);

stmt_test!(
    tuple_struct_with_multiple_fields,
    "struct Foo(int, str)",
    Struct::new(
        ident!("Foo"),
        vec![
            StructField {
                name: FieldKey::Int(0),
                annotation: Annotation::Kw(TyKw::Int),
                location: shared::Location::default(),
                public: false,
            },
            StructField {
                name: FieldKey::Int(1),
                annotation: Annotation::Kw(TyKw::Str),
                location: shared::Location::default(),
                public: false,
            },
        ],
    )
    .into_item()
);

stmt_test!(r#enum, "enum Foo {}", enu!(Foo {}).into_item());

stmt_test!(
    enum_with_fields,
    "enum Foo {
        Bar,
        Fizz,
    }",
    enu!(Foo {
        enum_struct!(Bar),
        enum_struct!(Fizz)
    })
    .into_item()
);

stmt_test!(
    enum_with_struct_fields,
    "enum Foo {
        Bar {
            fizz: int,
        },
        Buzz {
            baz: int,
        }
    }",
    enu!(Foo {
        enum_struct!(Bar {
            fizz: Annotation::Kw(TyKw::Int)
        }),
        enum_struct!(Buzz {
            baz: Annotation::Kw(TyKw::Int)
        })
    })
    .into_item()
);

stmt_test!(
    enum_with_tuple_fields,
    "enum Foo {
        Bar(),
        Buzz(int)
    }",
    enu!(Foo {
        enum_tuple!(Bar()),
        enum_tuple!(Buzz(Annotation::Kw(TyKw::Int)))
    })
    .into_item()
);

stmt_test!(r#struct, "struct Foo {}", struc!(Foo).into_item());

stmt_test!(struct_no_body, "struct Foo;", struc!(Foo).into_item());

stmt_test!(
    struct_with_fields,
    "struct Foo {
        bar: int,
        fizz: str,
    }",
    struc!(Foo {
        bar: Annotation::Kw(TyKw::Int),
        fizz: Annotation::Kw(TyKw::Str)
    })
    .into_item()
);

stmt_test!(
    struct_with_more_fields,
    "struct F {f: str}",
    struc!(F {
        f: Annotation::Kw(TyKw::Str)
    })
    .into_item()
);

stmt_test!(
    tuple_struct,
    "struct Foo(int)",
    struc!(Foo(Annotation::Kw(TyKw::Int))).into_item()
);

stmt_test!(
    function,
    "fn foo() {}",
    Function::new(ident!("foo"), vec![], None, block!()).into_item()
);

stmt_test!(
    method,
    "fn foo(self) {}",
    Function {
        name: ident!("foo"),
        parameters: vec![Binding::new(ident!("self"))],
        return_type: None,
        body: block!(),
    }
    .into_item()
);

stmt_test!(
    function_with_parameters,
    "fn foo(bar, baz) {}",
    Function::new(
        ident!("foo"),
        vec![Binding::new(ident!("bar")), Binding::new(ident!("baz")),],
        None,
        block!(),
    )
    .into_item()
);

stmt_test!(
    function_with_typed_parameters,
    "fn foo(bar: int, baz: str) {}",
    Function::new(
        ident!("foo"),
        vec![
            Binding::new(ident!("bar")).with_annotation(Annotation::Kw(TyKw::Int)),
            Binding::new(ident!("baz")).with_annotation(Annotation::Kw(TyKw::Str)),
        ],
        None,
        block!(),
    )
    .into_item()
);

stmt_test!(
    function_with_return_type,
    "fn foo(bar, baz) -> () {}",
    Function::new(
        ident!("foo"),
        vec![Binding::new(ident!("bar")), Binding::new(ident!("baz")),],
        Some(Annotation::Unit),
        block!(),
    )
    .into_item()
);

stmt_test!(
    function_with_default_parameters,
    "fn foo(bar=1, baz) {}",
    Function::new(
        ident!("foo"),
        vec![
            Binding::new(ident!("bar")).with_initilization(int!(1)),
            Binding::new(ident!("baz")),
        ],
        None,
        block!(),
    )
    .into_item()
);

stmt_test!(
    function_with_everything,
    r#"fn foo(bar: int = 1, baz: str = "hi") -> int {}"#,
    Function::new(
        ident!("foo"),
        vec![
            Binding::new(ident!("bar"))
                .with_initilization(int!(1))
                .with_annotation(Annotation::Kw(TyKw::Int)),
            Binding::new(ident!("baz"))
                .with_initilization(Literal::String("hi".to_string()).into_expr())
                .with_annotation(Annotation::Kw(TyKw::Str)),
        ],
        Some(Annotation::Kw(TyKw::Int)),
        block!(),
    )
    .into_item()
);
