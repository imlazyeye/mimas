use crate::{
    IntoExpr, IntoStmt, block, components::*, expr::*, ident, ident_expr, int, item::*, lex::TyKw,
    stmt::*,
};
use pretty_assertions::assert_eq;
use shared::Location;

macro_rules! expr_test {
    ($name:ident, $source:expr, $expected:expr) => {
        #[cfg(test)]
        #[test]
        fn $name() {
            let expected: crate::expr::Expr = crate::expr::ExprKind::from($expected).into_expr();
            let lexer = crate::lex::Lexer::new($source, 0, "test".into());
            let mut parser = crate::Parser::new(lexer);
            let output = parser.expr().unwrap();
            if output.kind() != expected.kind() {
                assert_eq!(output.to_string(), expected.to_string());
                assert_eq!(output.kind(), expected.kind());
                panic!();
            }
        }
    };
}

expr_test!(r#loop, "loop {}", Loop::new(block!()));

expr_test!(
    loop_no_block,
    "loop break",
    Loop::new(Break::new(None).into_expr())
);

expr_test!(return_expr_no_value, "return", Return::new(None));

expr_test!(
    return_expr_with_value,
    "return 0",
    Return::new(Some(int!(0)))
);

expr_test!(break_expr_no_value, "break", Break::new(None));

expr_test!(break_expr_with_value, "break 0", Break::new(Some(int!(0))));

expr_test!(collect_expr, "collect 0", Collect::new(int!(0)));

expr_test!(
    r#while,
    "while true {}",
    While::new(Literal::True.into_expr(), block!())
);

expr_test!(
    while_no_block,
    "while true break;",
    While::new(Literal::True.into_expr(), Break::new(None).into_expr())
);

expr_test!(
    while_let,
    "while let a = foo {}",
    While::new_with_pat(
        Pat::new(PatKind::Ident(ident!("a")), shared::Location::default(),),
        ident_expr!("foo"),
        block!()
    )
);

expr_test!(
    while_let_tuple,
    "while let (a, b) = foo {}",
    While::new_with_pat(
        Pat::new(
            PatKind::Tuple(vec![ident!("a").into(), ident!("b").into()]),
            shared::Location::default(),
        ),
        ident_expr!("foo"),
        block!()
    )
);

expr_test!(
    for_in_no_block,
    "for a in b if a a else null",
    For::new(
        ident!("a"),
        ident_expr!("b"),
        If::new_with_else(
            ident_expr!("a"),
            ident_expr!("a"),
            Literal::Null.into_expr()
        )
        .into_expr()
    )
);

expr_test!(
    for_in_with_block,
    "for a in b {}",
    For::new(ident!("a"), ident_expr!("b"), block!())
);

expr_test!(
    collection_no_block,
    "for a in b collect a",
    For::new(
        ident!("a"),
        ident_expr!("b"),
        Collect::new(ident_expr!("a")).into_expr()
    )
);

expr_test!(
    collection_with_block,
    "for a in b { collect a; }",
    For::new(
        ident!("a"),
        ident_expr!("b"),
        block!(Collect::new(ident_expr!("a")).into_expr().into_stmt())
    )
);

expr_test!(
    for_tuple_binding,
    "for (a, b) in z {}",
    For::new(
        Pat::new(
            PatKind::Tuple(vec![ident!("a").into(), ident!("b").into()]),
            shared::Location::default(),
        ),
        ident_expr!("z"),
        block!()
    )
);

expr_test!(
    for_range,
    "for a in 0..1 {}",
    For::new(
        ident!("a"),
        Range::new(
            Literal::Int(0).into_expr(),
            Literal::Int(1).into_expr(),
            false
        )
        .into_expr(),
        block!()
    )
);

expr_test!(
    for_range_inclusive,
    "for a in 0..=1 {}",
    For::new(
        ident!("a"),
        Range::new(
            Literal::Int(0).into_expr(),
            Literal::Int(1).into_expr(),
            true
        )
        .into_expr(),
        block!()
    )
);

expr_test!(r#if, "if a {}", If::new(ident_expr!("a"), block!()));

expr_test!(
    if_else,
    "if a {} else {}",
    If::new_with_else(ident_expr!("a"), block!(), block!())
);

expr_test!(
    if_no_block,
    "if a b else c",
    If::new_with_else(ident_expr!("a"), ident_expr!("b"), ident_expr!("c"))
);

expr_test!(
    if_let,
    "if let a = b {}",
    If {
        condition: ident_expr!("b"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(PatKind::Ident(ident!("a")), Location::SYNTHETIC)),
    }
);

expr_test!(
    if_let_tuple,
    "if let (a, b) = c {}",
    If {
        condition: ident_expr!("c"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(
            PatKind::Tuple(vec![ident!("a").into(), ident!("b").into()]),
            Location::default(),
        )),
    }
);

expr_test!(
    if_let_literal_int,
    "if let 5 = n {}",
    If {
        condition: ident_expr!("n"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(
            PatKind::Literal(Literal::Int(5)),
            Location::default(),
        )),
    }
);

expr_test!(
    if_let_literal_bool,
    "if let true = b {}",
    If {
        condition: ident_expr!("b"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(
            PatKind::Literal(Literal::True),
            Location::default(),
        )),
    }
);

expr_test!(
    if_let_literal_str,
    r#"if let "hi" = s {}"#,
    If {
        condition: ident_expr!("s"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(
            PatKind::Literal(Literal::String("hi".into())),
            Location::default(),
        )),
    }
);

expr_test!(
    if_let_null_bind,
    "if let a? = b {}",
    If {
        condition: ident_expr!("b"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(
            PatKind::NullBind(Box::new(ident!("a").into())),
            Location::default(),
        )),
    }
);

expr_test!(
    if_let_null_bind_str,
    r#"if let "hi"? = s {}"#,
    If {
        condition: ident_expr!("s"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(
            PatKind::NullBind(Box::new(Pat::new(
                PatKind::Literal(Literal::String("hi".into())),
                Location::default(),
            ))),
            Location::default(),
        )),
    }
);

expr_test!(
    if_let_null_bind_int,
    "if let 42? = n {}",
    If {
        condition: ident_expr!("n"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(
            PatKind::NullBind(Box::new(Pat::new(
                PatKind::Literal(Literal::Int(42)),
                Location::default(),
            ))),
            Location::default(),
        )),
    }
);

expr_test!(
    if_let_null_bind_false,
    "if let false? = b {}",
    If {
        condition: ident_expr!("b"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(
            PatKind::NullBind(Box::new(Pat::new(
                PatKind::Literal(Literal::False),
                Location::default(),
            ))),
            Location::default(),
        )),
    }
);

expr_test!(
    if_let_null_bind_struct,
    "if let Pair { a }? = p {}",
    If {
        condition: ident_expr!("p"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(
            PatKind::NullBind(Box::new(Pat::new(
                PatKind::Struct(Box::new(ident_expr!("Pair")), {
                    let mut m = hashbrown::HashMap::new();
                    m.insert("a".into(), Pat::from(ident!("a")));
                    m
                }),
                Location::default(),
            ))),
            Location::default(),
        )),
    }
);

expr_test!(
    if_let_or,
    "if let 0 | 1 = n {}",
    If {
        condition: ident_expr!("n"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(
            PatKind::Or(vec![
                Pat::new(PatKind::Literal(Literal::Int(0)), Location::default()),
                Pat::new(PatKind::Literal(Literal::Int(1)), Location::default()),
            ]),
            Location::default(),
        )),
    }
);

expr_test!(
    if_let_struct,
    "if let Pair { a, b } = p {}",
    If {
        condition: ident_expr!("p"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(
            PatKind::Struct(Box::new(ident_expr!("Pair")), {
                let mut m = hashbrown::HashMap::new();
                m.insert("a".into(), Pat::from(ident!("a")));
                m.insert("b".into(), Pat::from(ident!("b")));
                m
            }),
            Location::default(),
        )),
    }
);

expr_test!(
    if_let_tuple_struct,
    "if let Wrap(x) = w {}",
    If {
        condition: ident_expr!("w"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(
            PatKind::TupleVariant(Box::new(ident_expr!("Wrap")), vec![ident!("x").into()]),
            Location::default(),
        )),
    }
);

expr_test!(
    if_let_enum_unit_variant,
    "if let Color::Red = c {}",
    If {
        condition: ident_expr!("c"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(
            PatKind::Variant(Box::new(
                Access::DoubleColon {
                    left: ident_expr!("Color"),
                    right: ident!("Red"),
                }
                .into_expr(),
            )),
            Location::default(),
        )),
    }
);

expr_test!(
    if_let_enum_tuple_variant,
    "if let Shape::Circle(r) = s {}",
    If {
        condition: ident_expr!("s"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(
            PatKind::TupleVariant(
                Box::new(
                    Access::DoubleColon {
                        left: ident_expr!("Shape"),
                        right: ident!("Circle"),
                    }
                    .into_expr(),
                ),
                vec![ident!("r").into()],
            ),
            Location::default(),
        )),
    }
);

expr_test!(
    if_let_enum_struct_variant,
    "if let Msg::Move { x, y } = m {}",
    If {
        condition: ident_expr!("m"),
        main_body: block!(),
        else_expr: None,
        binding: Some(Pat::new(
            PatKind::Struct(
                Box::new(
                    Access::DoubleColon {
                        left: ident_expr!("Msg"),
                        right: ident!("Move"),
                    }
                    .into_expr(),
                ),
                {
                    let mut m = hashbrown::HashMap::new();
                    m.insert("x".into(), Pat::from(ident!("x")));
                    m.insert("y".into(), Pat::from(ident!("y")));
                    m
                },
            ),
            Location::default(),
        )),
    }
);

expr_test!(
    r#match,
    "match foo {}",
    Match::new(ident_expr!("foo"), vec![], false)
);

expr_test!(
    match_with_one_case,
    "match foo {
        0 => {}
    }",
    Match::new(
        ident_expr!("foo"),
        vec![MatchCase::new(
            crate::components::Pat::new(
                crate::components::PatKind::Literal(crate::expr::Literal::Int(0)),
                shared::Location::default()
            ),
            None,
            block!()
        )],
        false
    )
);

expr_test!(
    match_with_multiple_cases,
    "match foo {
        0 => {},
        1 => {},
    }",
    Match::new(
        ident_expr!("foo"),
        vec![
            MatchCase::new(
                crate::components::Pat::new(
                    crate::components::PatKind::Literal(crate::expr::Literal::Int(0)),
                    shared::Location::default()
                ),
                None,
                block!()
            ),
            MatchCase::new(
                crate::components::Pat::new(
                    crate::components::PatKind::Literal(crate::expr::Literal::Int(1)),
                    shared::Location::default()
                ),
                None,
                block!()
            ),
        ],
        false,
    )
);

expr_test!(
    match_with_fallback,
    "match foo {
        _ => {},
    }",
    Match::new(
        ident_expr!("foo"),
        vec![MatchCase::new(
            crate::components::Pat::from(crate::expr::Ident::synthetic("_")),
            None,
            block!()
        )],
        false,
    )
);

expr_test!(block, "{}", Block::new(vec![]));

expr_test!(
    block_yield_value,
    "{ 0 }",
    Block::new_with_yield(vec![], int!(0))
);

expr_test!(
    multi_statement_block_yield,
    "{
        let a = 0;
        let b = 0;
        a + b   
    }",
    Block::new_with_yield(
        vec![
            Let::new(
                Pat::new(PatKind::Ident(ident!("a")), shared::Location::default()),
                int!(0),
                None
            )
            .into_stmt(),
            Let::new(
                Pat::new(PatKind::Ident(ident!("b")), shared::Location::default()),
                int!(0),
                None
            )
            .into_stmt(),
        ],
        Evaluation::new(ident_expr!("a"), EvaluationOp::Plus, ident_expr!("b")).into_expr()
    )
);

expr_test!(
    block_in_block,
    "{{0}}",
    Block::new_with_yield(vec![], Block::new_with_yield(vec![], int!(0)).into_expr())
);

expr_test!(
    dictionary_block_mix,
    "{~{foo = {}}}",
    Block::new_with_yield(
        vec![],
        Literal::Dictionary(vec![(ident!("foo"), block!())]).into_expr()
    )
);

expr_test!(
    and,
    "1 && 1",
    Logical::new(int!(1), LogicalOp::And, int!(1))
);

expr_test!(or, "1 || 1", Logical::new(int!(1), LogicalOp::Or, int!(1)));

expr_test!(
    addition,
    "1 + 1",
    Evaluation::new(int!(1), EvaluationOp::Plus, int!(1))
);

expr_test!(
    subtraction,
    "1 - 1",
    Evaluation::new(int!(1), EvaluationOp::Minus, int!(1))
);

expr_test!(
    multiplication,
    "1 * 1",
    Evaluation::new(int!(1), EvaluationOp::Multiply, int!(1))
);

expr_test!(
    division,
    "1 / 1",
    Evaluation::new(int!(1), EvaluationOp::Divide, int!(1))
);

expr_test!(
    modulo,
    "1 % 1",
    Evaluation::new(int!(1), EvaluationOp::Modulo, int!(1))
);

expr_test!(
    div,
    "1 ~/ 1",
    Evaluation::new(int!(1), EvaluationOp::Div, int!(1))
);

expr_test!(
    bitwise_and,
    "1 & 1",
    Evaluation::new(int!(1), EvaluationOp::And, int!(1))
);

expr_test!(
    bitwise_or,
    "1 | 1",
    Evaluation::new(int!(1), EvaluationOp::Or, int!(1))
);

expr_test!(
    bitwise_chain,
    "1 | 1 | 1",
    Evaluation::new(
        Evaluation::new(int!(1), EvaluationOp::Or, int!(1)).into_expr(),
        EvaluationOp::Or,
        int!(1),
    )
);

expr_test!(
    bitwise_xor,
    "1 ^ 1",
    Evaluation::new(int!(1), EvaluationOp::Xor, int!(1))
);

expr_test!(
    dot_access_bitwise,
    "foo.bar | foo.bar",
    Evaluation::new(
        Access::Dot {
            left: ident_expr!("foo"),
            right: ident_expr!("bar"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        EvaluationOp::Or,
        Access::Dot {
            left: ident_expr!("foo"),
            right: ident_expr!("bar"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
    )
);

expr_test!(
    combo_math,
    "1 * 1 + 1 >> 1 & 1 == 1",
    Equality::new(
        Evaluation::new(
            Evaluation::new(
                Evaluation::new(
                    Evaluation::new(int!(1), EvaluationOp::Multiply, int!(1)).into_expr(),
                    EvaluationOp::Plus,
                    int!(1),
                )
                .into_expr(),
                EvaluationOp::BitShiftRight,
                int!(1),
            )
            .into_expr(),
            EvaluationOp::And,
            int!(1),
        )
        .into_expr(),
        EqualityOp::Equal,
        int!(1),
    )
);

expr_test!(
    math_on_block,
    "1 / { 1 }",
    Evaluation::new(
        int!(1),
        EvaluationOp::Divide,
        Block::new_with_yield(vec![], int!(1)).into_expr()
    )
);

expr_test!(
    bit_shift_left,
    "1 << 1",
    Evaluation::new(int!(1), EvaluationOp::BitShiftLeft, int!(1))
);

expr_test!(
    bit_shift_right,
    "1 >> 1",
    Evaluation::new(int!(1), EvaluationOp::BitShiftRight, int!(1))
);

expr_test!(
    less_than,
    "1 < 1",
    Equality::new(int!(1), EqualityOp::Less, int!(1))
);

expr_test!(
    less_than_or_equal,
    "1 <= 1",
    Equality::new(int!(1), EqualityOp::LessOrEqual, int!(1),)
);

expr_test!(
    greater_than,
    "1 > 1",
    Equality::new(int!(1), EqualityOp::Greater, int!(1),)
);

expr_test!(
    greater_than_or_equal,
    "1 >= 1",
    Equality::new(int!(1), EqualityOp::GreaterOrEqual, int!(1),)
);

expr_test!(
    equal,
    "1 == 1",
    Equality::new(int!(1), EqualityOp::Equal, int!(1))
);

expr_test!(
    bang_equal,
    "1 != 1",
    Equality::new(int!(1), EqualityOp::NotEqual, int!(1))
);

expr_test!(
    null_coalecence,
    "foo ?? 1",
    Coalescence::new(ident_expr!("foo"), int!(1))
);

expr_test!(not, "!foo", Unary::new(UnaryOp::Not, ident_expr!("foo")));

expr_test!(positive, "+1", Unary::new(UnaryOp::Positive, int!(1)));

expr_test!(negative, "-1", Unary::new(UnaryOp::Negative, int!(1)));

expr_test!(
    dot_unary,
    "!foo.bar",
    Unary::new(
        UnaryOp::Not,
        Access::Dot {
            left: ident_expr!("foo"),
            right: ident_expr!("bar"),
            kind: AccessKind::Direct
        }
        .into_expr(),
    )
);

expr_test!(
    ds_unary,
    "!foo[bar]",
    Unary::new(
        UnaryOp::Not,
        Access::Square {
            left: ident_expr!("foo"),
            key: ident_expr!("bar"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
    )
);

expr_test!(bitwise_not, "~1", Unary::new(UnaryOp::BitwiseNot, int!(1)));

expr_test!(
    fstring_literal_only,
    "f\"hello\"",
    FString::new(vec![FStringPart::Literal("hello".to_string())])
);

expr_test!(
    fstring_with_expr,
    "f\"{a}\"",
    FString::new(vec![FStringPart::Expr(ident_expr!("a"))])
);

expr_test!(
    fstring_mixed,
    "f\"foo {a} bar\"",
    FString::new(vec![
        FStringPart::Literal("foo ".to_string()),
        FStringPart::Expr(ident_expr!("a")),
        FStringPart::Literal(" bar".to_string()),
    ])
);

// `{{` and `}}` are escape sequences for literal braces (python/rust convention).
expr_test!(
    fstring_escaped_braces,
    "f\"{{x}}\"",
    FString::new(vec![FStringPart::Literal("{x}".to_string())])
);

expr_test!(
    fstring_braces_around_interp,
    "f\"{{{a}}}\"",
    FString::new(vec![
        FStringPart::Literal("{".to_string()),
        FStringPart::Expr(ident_expr!("a")),
        FStringPart::Literal("}".to_string()),
    ])
);

// nested `"..."` inside an interp expression must not terminate the outer f-string
expr_test!(
    fstring_interp_with_string_index,
    "f\"{m[\"k\"]}\"",
    FString::new(vec![FStringPart::Expr(
        Access::Square {
            left: ident_expr!("m"),
            key: ExprKind::Literal(Literal::String("k".into())).into_expr(),
            kind: AccessKind::Direct,
        }
        .into_expr(),
    )])
);

// a `}` inside a nested string in the interp must not close the interp early
expr_test!(
    fstring_interp_with_brace_in_string,
    "f\"{[\"}\"]}\"",
    FString::new(vec![FStringPart::Expr(
        ExprKind::Literal(Literal::Array(vec![
            ExprKind::Literal(Literal::String("}".into())).into_expr(),
        ]))
        .into_expr(),
    )])
);

expr_test!(
    dot_access_call,
    "a.b()",
    Call::new(
        Access::Dot {
            left: ident_expr!("a"),
            right: ident_expr!("b"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        vec![]
    )
);

expr_test!(
    struct_literal,
    "Foo {}",
    Literal::Struct(StructLiteral {
        name: ident_expr!("Foo"),
        fields: vec![]
    })
);

expr_test!(
    struct_literal_with_fields,
    "Foo { a = 0 }",
    Literal::Struct(StructLiteral {
        name: ident_expr!("Foo"),
        fields: vec![(FieldKey::Ident(ident!("a")), int!(0))]
    })
);

expr_test!(
    struct_literal_inferred,
    "Foo { a }",
    Literal::Struct(StructLiteral {
        name: ident_expr!("Foo"),
        fields: vec![(FieldKey::Ident(ident!("a")), ident!("a").into_expr())]
    })
);

expr_test!(
    struct_literal_inferred_trail,
    "Foo { a, }",
    Literal::Struct(StructLiteral {
        name: ident_expr!("Foo"),
        fields: vec![(FieldKey::Ident(ident!("a")), ident!("a").into_expr())]
    })
);

expr_test!(
    struct_literal_inferred_multi,
    "Foo { a, b }",
    Literal::Struct(StructLiteral {
        name: ident_expr!("Foo"),
        fields: vec![
            (FieldKey::Ident(ident!("a")), ident!("a").into_expr()),
            (FieldKey::Ident(ident!("b")), ident!("b").into_expr()),
        ]
    })
);

expr_test!(
    struct_literal_inferred_mixed,
    "Foo { a, b = 0 }",
    Literal::Struct(StructLiteral {
        name: ident_expr!("Foo"),
        fields: vec![
            (FieldKey::Ident(ident!("a")), ident!("a").into_expr()),
            (FieldKey::Ident(ident!("b")), int!(0)),
        ]
    })
);

expr_test!(
    enum_literal,
    "Foo::Bar {}",
    Literal::Struct(StructLiteral {
        name: Access::DoubleColon {
            left: ident_expr!("Foo"),
            right: ident!("Bar")
        }
        .into_expr(),
        fields: vec![]
    })
);

expr_test!(
    enum_literal_with_fields,
    "Foo::Bar { a = 0 }",
    Literal::Struct(StructLiteral {
        name: Access::DoubleColon {
            left: ident_expr!("Foo"),
            right: ident!("Bar")
        }
        .into_expr(),
        fields: vec![(FieldKey::Ident(ident!("a")), int!(0))]
    })
);

expr_test!(closure, "|| {}", Closure::new(vec![], block!()));

expr_test!(
    closure_with_parameters,
    "|a: int, b| {}",
    Closure::new(
        vec![
            Binding::new(ident!("a")).with_annotation(Annotation::Kw(TyKw::Int)),
            Binding::new(ident!("b")),
        ],
        block!()
    )
);

expr_test!(
    call_on_array,
    "a[0]()",
    Call::new(
        Access::Square {
            left: ident_expr!("a"),
            key: int!(0),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        vec![]
    )
);

expr_test!(call, "foo()", Call::new(ident_expr!("foo"), vec![]));

expr_test!(
    call_with_args,
    "foo(0, 1, 2)",
    Call::new(
        ident_expr!("foo"),
        vec![
            Argument::new(int!(0)),
            Argument::new(int!(1)),
            Argument::new(int!(2)),
        ]
    )
);

expr_test!(
    call_with_named_arg,
    "foo(a=5)",
    Call::new(
        ident_expr!("foo"),
        vec![Argument::named(ident!("a"), int!(5))]
    )
);

expr_test!(
    call_trailing_commas,
    "foo(0, 1, 2)",
    Call::new(
        ident_expr!("foo"),
        vec![
            Argument::new(int!(0)),
            Argument::new(int!(1)),
            Argument::new(int!(2)),
        ]
    )
);

expr_test!(
    call_identity,
    "self.bar()",
    Call::new(
        Access::Dot {
            left: Ident {
                lexeme: "self".into(),
                location: Default::default(),
            }
            .into_expr(),
            right: ident_expr!("bar"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        vec![],
    )
);

expr_test!(empty_array, "[]", ExprKind::Literal(Literal::Array(vec![])));

expr_test!(
    simple_array,
    "[0, 1, 2]",
    ExprKind::Literal(Literal::Array(vec![int!(0), int!(1), int!(2),]))
);

expr_test!(
    array_comma_trail,
    "[0, 1, 2,]",
    ExprKind::Literal(Literal::Array(vec![int!(0), int!(1), int!(2),]))
);

expr_test!(
    simple_tuple,
    "(0,)",
    ExprKind::Literal(Literal::Tuple(vec![int!(0)]))
);

expr_test!(
    multi_tuple,
    "(0, 1)",
    ExprKind::Literal(Literal::Tuple(vec![int!(0), int!(1)]))
);

expr_test!(
    tuple_comma_trail,
    "(0, 1,)",
    ExprKind::Literal(Literal::Tuple(vec![int!(0), int!(1)]))
);

expr_test!(
    empty_dictionary,
    "~{}",
    ExprKind::Literal(Literal::Dictionary(vec![]))
);

expr_test!(
    filled_dictionary,
    "~{ foo = bar, fizz = buzz }",
    ExprKind::Literal(Literal::Dictionary(vec![
        (ident!("foo"), ident_expr!("bar")),
        (ident!("fizz"), ident_expr!("buzz")),
    ]))
);

expr_test!(
    dictionary_comma_trail,
    "~{ foo = bar, fizz = buzz, }",
    ExprKind::Literal(Literal::Dictionary(vec![
        (ident!("foo"), ident_expr!("bar")),
        (ident!("fizz"), ident_expr!("buzz")),
    ]))
);

expr_test!(
    square_access,
    "foo[bar]",
    Access::Square {
        left: ident_expr!("foo"),
        key: ident_expr!("bar"),
        kind: AccessKind::Direct,
    }
);

expr_test!(
    chained_square_accesses,
    "foo[bar][buzz]",
    Access::Square {
        left: Access::Square {
            left: ident_expr!("foo"),
            key: ident_expr!("bar"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        key: ident_expr!("buzz"),
        kind: AccessKind::Direct,
    }
);

expr_test!(
    option_square_accesses,
    "foo?[buzz]",
    Access::Square {
        left: ident_expr!("foo"),
        key: ident_expr!("buzz"),
        kind: AccessKind::Option,
    }
);

expr_test!(
    square_access_call,
    "foo[bar]()",
    Call::new(
        Access::Square {
            left: ident_expr!("foo"),
            key: ident_expr!("bar"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        vec![],
    )
);

expr_test!(
    option_square_access_call,
    "foo?[bar]()",
    Call::new(
        Access::Square {
            left: ident_expr!("foo"),
            key: ident_expr!("bar"),
            kind: AccessKind::Option,
        }
        .into_expr(),
        vec![],
    )
);

expr_test!(
    identity_access,
    "self.bar",
    Access::Dot {
        left: Ident {
            lexeme: "self".into(),
            location: Default::default(),
        }
        .into_expr(),
        right: ident_expr!("bar"),
        kind: AccessKind::Direct,
    }
);

expr_test!(
    direct_dot_access,
    "foo.bar",
    Access::Dot {
        left: ident_expr!("foo"),
        right: ident_expr!("bar"),
        kind: AccessKind::Direct,
    }
);

expr_test!(
    tuple_dot_access,
    "foo.1",
    Access::Dot {
        left: ident_expr!("foo"),
        right: int!(1),
        kind: AccessKind::Direct,
    }
);

expr_test!(
    option_dot_access,
    "foo?.bar",
    Access::Dot {
        left: ident_expr!("foo"),
        right: ident_expr!("bar"),
        kind: AccessKind::Option,
    }
);

expr_test!(
    chained_dot_access,
    "foo.bar.buzz",
    Access::Dot {
        left: Access::Dot {
            left: ident_expr!("foo"),
            right: ident_expr!("bar"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        right: ident_expr!("buzz"),
        kind: AccessKind::Direct,
    }
);

expr_test!(
    chained_option_dot_access,
    "foo.bar?.fizz?.buzz",
    Access::Dot {
        left: Access::Dot {
            left: Access::Dot {
                left: ident_expr!("foo"),
                right: ident_expr!("bar"),
                kind: AccessKind::Direct,
            }
            .into_expr(),
            right: ident_expr!("fizz"),
            kind: AccessKind::Option,
        }
        .into_expr(),
        right: ident_expr!("buzz"),
        kind: AccessKind::Option,
    }
);

expr_test!(
    direct_dot_access_to_call,
    "foo.bar()",
    Call::new(
        Access::Dot {
            left: ident_expr!("foo"),
            right: ident_expr!("bar"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        vec![],
    )
);

expr_test!(
    tuple_index_then_method,
    "foo.0.bar()",
    Call::new(
        Access::Dot {
            left: Access::Dot {
                left: ident_expr!("foo"),
                right: int!(0),
                kind: AccessKind::Direct,
            }
            .into_expr(),
            right: ident_expr!("bar"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        vec![],
    )
);

expr_test!(
    tuple_index_then_field,
    "foo.0.bar",
    Access::Dot {
        left: Access::Dot {
            left: ident_expr!("foo"),
            right: int!(0),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        right: ident_expr!("bar"),
        kind: AccessKind::Direct,
    }
);

expr_test!(
    indirect_dot_access_to_call,
    "foo?.bar()",
    Call::new(
        Access::Dot {
            left: ident_expr!("foo"),
            right: ident_expr!("bar"),
            kind: AccessKind::Option,
        }
        .into_expr(),
        vec![],
    )
);

expr_test!(
    direct_dot_access_to_square_access,
    "foo.bar[0]",
    Access::Square {
        left: Access::Dot {
            left: ident_expr!("foo"),
            right: ident_expr!("bar"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        key: int!(0),
        kind: AccessKind::Direct,
    }
);

expr_test!(
    option_dot_access_to_square_access,
    "foo?.bar[0]",
    Access::Square {
        left: Access::Dot {
            left: ident_expr!("foo"),
            right: ident_expr!("bar"),
            kind: AccessKind::Option,
        }
        .into_expr(),
        key: int!(0),
        kind: AccessKind::Direct,
    }
);

expr_test!(
    direct_dot_access_from_call,
    "foo().bar",
    Access::Dot {
        left: Call::new(ident_expr!("foo"), vec![]).into_expr(),
        right: ident_expr!("bar"),
        kind: AccessKind::Direct,
    }
);

expr_test!(
    indirect_dot_access_from_call,
    "foo()?.bar",
    Access::Dot {
        left: Call::new(ident_expr!("foo"), vec![]).into_expr(),
        right: ident_expr!("bar"),
        kind: AccessKind::Option,
    }
);

expr_test!(
    chained_calls,
    "foo().bar()",
    Call::new(
        Access::Dot {
            left: Call::new(ident_expr!("foo"), vec![]).into_expr(),
            right: ident_expr!("bar"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        vec![],
    )
);

expr_test!(
    chain_calls_with_call_parameter,
    "foo().bar(buzz())",
    Call::new(
        Access::Dot {
            left: Call::new(ident_expr!("foo"), vec![]).into_expr(),
            right: ident_expr!("bar"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        vec![Argument::new(
            Call::new(ident_expr!("buzz"), vec![]).into_expr()
        )],
    )
);

expr_test!(
    square_dot_access,
    "foo[0].bar",
    Access::Dot {
        left: Access::Square {
            left: ident_expr!("foo"),
            key: int!(0),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        right: ident_expr!("bar"),
        kind: AccessKind::Direct,
    }
);

expr_test!(
    double_colon_access,
    "foo::bar",
    Access::DoubleColon {
        left: ident_expr!("foo"),
        right: ident!("bar"),
    }
);

expr_test!(
    double_colon_call,
    "foo::bar()",
    Call::new(
        Access::DoubleColon {
            left: ident_expr!("foo"),
            right: ident!("bar"),
        }
        .into_expr(),
        vec![]
    )
);

expr_test!(
    multiple_double_colon_access,
    "foo::bar::buzz",
    Access::DoubleColon {
        left: Access::DoubleColon {
            left: ident_expr!("foo"),
            right: ident!("bar"),
        }
        .into_expr(),
        right: ident!("buzz")
    }
);

expr_test!(
    unwrap,
    "foo!",
    Unwrap {
        expr: ident_expr!("foo")
    }
);

expr_test!(
    unwrap_access,
    "foo!.bar",
    Access::Dot {
        left: Unwrap {
            expr: ident_expr!("foo"),
        }
        .into_expr(),
        right: ident_expr!("bar"),
        kind: AccessKind::Direct
    }
);

expr_test!(
    unwrap_member,
    "foo.bar!",
    Unwrap {
        expr: Access::Dot {
            left: ident_expr!("foo"),
            right: ident_expr!("bar"),
            kind: AccessKind::Direct
        }
        .into_expr()
    }
);

expr_test!(
    unwrap_call,
    "foo()!",
    Unwrap {
        expr: Call::new(ident_expr!("foo"), vec![]).into_expr()
    }
);

expr_test!(filled_grouping, "(0)", Grouping::lazy(int!(0)));
expr_test!(
    nested_grouping,
    "((0) * 0)",
    Grouping::lazy(
        Evaluation::new(
            Grouping::lazy(int!(0)).into_expr(),
            EvaluationOp::Multiply,
            int!(0),
        )
        .into_expr(),
    )
);

expr_test!(r#true, "true", ExprKind::Literal(Literal::True));
expr_test!(r#false, "false", ExprKind::Literal(Literal::False));
expr_test!(null, "null", ExprKind::Literal(Literal::Null));
expr_test!(unit, "()", ExprKind::Literal(Literal::Unit));
expr_test!(ident, "foo", ident!("foo"));
expr_test!(int, "0", Literal::Int(0));
expr_test!(float, "0.1", Literal::Float(0.1));

expr_test!(
    string,
    "\"foo\"",
    ExprKind::Literal(Literal::String("foo".into()))
);

expr_test!(
    multi_line_string,
    "\"foo\nfoo\"",
    ExprKind::Literal(Literal::String("foo\nfoo".into()))
);
expr_test!(
    oh_x_hex,
    "0xa0f9a0",
    ExprKind::Literal(Literal::Hex("a0f9a0".into()))
);

expr_test!(
    logically_joined_expressions,
    "foo == 1 && foo == 1 && foo == 1",
    Logical::new(
        Logical::new(
            Equality::new(ident_expr!("foo"), EqualityOp::Equal, int!(1)).into_expr(),
            LogicalOp::And,
            Equality::new(ident_expr!("foo"), EqualityOp::Equal, int!(1)).into_expr(),
        )
        .into_expr(),
        LogicalOp::And,
        Equality::new(ident_expr!("foo"), EqualityOp::Equal, int!(1)).into_expr(),
    )
);

expr_test!(
    comment_in_builder_chain,
    "
            foo()
            // nothing in here!
            .bar()
        ",
    Call::new(
        Access::Dot {
            left: Call::new(ident_expr!("foo"), vec![]).into_expr(),
            right: ident_expr!("bar"),
            kind: AccessKind::Direct,
        }
        .into_expr(),
        vec![],
    )
);

expr_test!(int_with_underscores, "1_000_000", Literal::Int(1_000_000));

expr_test!(
    float_with_underscores,
    "1_000.000_1",
    Literal::Float(1000.0001)
);

expr_test!(
    addition_chain,
    "1 + 2 + 3",
    Evaluation::new(
        Evaluation::new(int!(1), EvaluationOp::Plus, int!(2)).into_expr(),
        EvaluationOp::Plus,
        int!(3),
    )
);

expr_test!(
    add_then_mul_precedence,
    "1 + 2 * 3",
    Evaluation::new(
        int!(1),
        EvaluationOp::Plus,
        Evaluation::new(int!(2), EvaluationOp::Multiply, int!(3)).into_expr(),
    )
);

expr_test!(
    mul_then_add_precedence,
    "1 * 2 + 3",
    Evaluation::new(
        Evaluation::new(int!(1), EvaluationOp::Multiply, int!(2)).into_expr(),
        EvaluationOp::Plus,
        int!(3),
    )
);

expr_test!(
    bit_shift_left_chained,
    "1 << 2 << 3",
    Evaluation::new(
        Evaluation::new(int!(1), EvaluationOp::BitShiftLeft, int!(2)).into_expr(),
        EvaluationOp::BitShiftLeft,
        int!(3),
    )
);

expr_test!(
    chained_eq_chain,
    "1 == 1 == 1",
    Equality::new(
        Equality::new(int!(1), EqualityOp::Equal, int!(1)).into_expr(),
        EqualityOp::Equal,
        int!(1),
    )
);

expr_test!(
    nested_array,
    "[[1, 2], [3, 4]]",
    ExprKind::Literal(Literal::Array(vec![
        Literal::Array(vec![int!(1), int!(2)]).into_expr(),
        Literal::Array(vec![int!(3), int!(4)]).into_expr(),
    ]))
);

expr_test!(
    coalescence_chained,
    "a ?? b ?? c",
    Coalescence::new(
        ident_expr!("a"),
        Coalescence::new(ident_expr!("b"), ident_expr!("c"),).into_expr()
    )
);

expr_test!(
    coalescence_with_or,
    "a || b ?? c",
    Coalescence::new(
        Logical::new(ident_expr!("a"), LogicalOp::Or, ident_expr!("b")).into_expr(),
        ident_expr!("c"),
    )
);

expr_test!(
    call_named_after_positional,
    "foo(0, b=1)",
    Call::new(
        ident_expr!("foo"),
        vec![
            Argument::new(int!(0)),
            Argument::named(ident!("b"), int!(1))
        ]
    )
);

expr_test!(
    raise_str_literal,
    "raise \"bad\"",
    Raise::new(Literal::String("bad".into()).into_expr())
);

expr_test!(
    raise_expr_value,
    "raise foo()",
    Raise::new(Call::new(ident_expr!("foo"), vec![]).into_expr())
);

expr_test!(
    absolve_with_closure,
    "foo() absolve |e| 0",
    Absolve::new(
        Call::new(ident_expr!("foo"), vec![]).into_expr(),
        Closure::new(vec![Binding::new(ident!("e"))], int!(0)).into_expr(),
    )
);

expr_test!(membership, "1 in 1", In::new(int!(1), int!(1), true));

expr_test!(not_membership, "1 !in 1", In::new(int!(1), int!(1), false));
