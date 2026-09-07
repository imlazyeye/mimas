use crate::components::Ty::*;

test_ty!(
    adt,
    "struct Foo {}",
    "Foo" => adt!({})
);

// test_ty!(
//     tuple_adt,
//     "struct Foo(int",
//     "Foo" => adt!((Int))
// );

test_ty!(
    adt_no_body,
    "struct Foo;",
    "Foo" => adt!({})
);

test_ty!(
    adt_with_fields,
    "struct Foo {
        a: int,
        b: str
    }",
    "Foo" => adt!({
        a: Int,
        b: Str
    })
);

test_ty!(
    adt_by_name,
    "struct Foo {}",
    "Foo" => adt!({})
);

test_ty!(
    adt_as_type,
    "struct Foo {}
    let a: Foo = Foo {};",
    "Foo" => adt!({}),
    "a" => adt!({}),
);

test_ty!(
    nested_adts,
    "struct Foo {}
    struct Bar {
        foo: Foo,
    }
    struct Fizz {
        bar: Bar,
    }
    struct Buzz {
        foo: Foo,
        bar: Bar,
        fizz: Fizz,
    }",
    "Foo" => adt!({}),
    "Bar" => adt!({
        foo: query!(Foo)
    }),
    "Fizz" => adt!({
        bar: query!(Bar)
    }),
    "Buzz" => adt!({
        foo: query!(Foo),
        bar: query!(Bar),
        fizz: query!(Fizz)
    })
);

test_ty!(
    out_of_order_adts,
    "struct Foo { v: Bar }
    struct Bar {}",
    "Foo" => adt!({ v: query!(Bar) })
);

test_ty!(
    out_of_order_impl,
    "impl Bar {}
    struct Bar {}",
    "Bar" => adt!({})
);

test_ty!(
    adt_access,
    "struct Foo { bar: int }
    let foo = Foo { bar = 0 };",
    "foo.bar" => Int
);

test_ty!(
    adt_optional_access,
    "struct Foo { bar: int }
    let foo: Foo? = Foo { bar = 0 };",
    "foo?.bar" => option!(Int)
);

test_ty!(
    impl_adt_empty,
    "struct Foo {};
    impl Foo {}",
    "Foo" => adt!({}),
);

test_ty!(
    impl_adt_function,
    "struct Foo {};
    impl Foo {
        fn bar() {}
    }",
    "Foo" => adt!({
        bar: func!(() -> Unit)
    }),
);

test_ty!(
    adt_colon_access_const,
    "struct Foo {}
    impl Foo {
        const BAR = 0;
    }",
    "Foo::BAR" => Int,
);

test_ty!(
    adt_colon_access_fn,
    "struct Foo {}
    impl Foo {
        fn bar() -> int { 0 }
    }",
    "Foo::bar" => func!(() -> Int),
);

test_ty!(
    access_adt_constructor_on_other,
    "struct Foo {}
    struct Bar {
        fizz: int
    }
    impl Foo {
        fn bar() -> Bar {
            Bar { fizz = 0 }
        }
    }",
    "Bar" => adt!({ fizz: Int }),
    "Foo::bar" => func!(() -> query!(Bar)),
    "Foo::bar()" => query!(Bar),
);

test_ty!(
    robust,
    r#"struct MyCollection {
        inner: [int],
        cursor: int,
    }

    impl MyCollection {
        fn new() -> MyCollection {
            MyCollection { inner = [], cursor = 0 }
        }
    }

    fn foo() -> MyCollection {
        let collection = MyCollection::new();
        collection
    }"#,
    "MyCollection" => adt!({
        inner: array!(Int),
        cursor: Int,
        new: func!(() -> query!(MyCollection))
    }),
    "foo()" => query!(MyCollection),
);

test_fail!(
    colon_access_field,
    "struct Foo { bar: int }
    let a = Foo::bar;"
);

// Recursion/Identity
test_ty!(
    recursive_struct,
    "struct Foo { f: Foo }",
    "Foo" => adt!({ f: query!(Foo) })
);

test_ty!(
    nested_recursive_struct,
    "struct Foo { f: Foo? }",
    "Foo" => adt!({ f: option!(query!(Foo)) })
);

test_ty!(
    recursive_tuple_field,
    "struct Foo { f: (Foo, int) }",
    "Foo" => adt!({ f: tuple!(query!(Foo), Int) })
);

test_ty!(
    constructor,
    "struct Foo {}
    impl Foo {
        fn new() -> Foo {
            Foo {}
        }
    }
    let foo = Foo::new();",
    "foo" => query!(Foo),
);

test_ty!(
    constructor_self,
    "struct Foo { x: int }
    impl Foo {
        fn new(v: int) -> Self {
            Self { x = v }
        }
    }
    let foo = Foo::new(7);",
    "foo.x" => Int,
);

test_fail!(
    use_spawned_adt_as_struct,
    "struct Foo { x: int }
    let foo = Foo { x = 0 };
    let bar = foo { x = 1 };"
);

test_ty!(
    field_array,
    "struct Foo { a: [int] }
     let foo = Foo { a = [1, 2, 3] };",
    "foo.a" => array!(Int)
);

test_ty!(
    field_optional,
    "struct Foo { a: int? }
     let foo = Foo { a = null };",
    "foo.a" => option!(Int)
);

test_ty!(
    field_assignment,
    "struct Foo { a: int }
     let foo = Foo { a = 0 };
     foo.a = 1;",
    "foo.a" => Int
);

test_fail!(
    field_wrong_type,
    "struct Foo { a: int }
     let foo = Foo { a = true };"
);

test_fail!(
    extra_field,
    "struct Foo { a: int }
     let foo = Foo { a = 0, b = 1 };"
);

test_fail!(
    read_nonexistent_field,
    "struct Foo { a: int }
     let foo = Foo { a = 0 };
     let b = foo.b;"
);

test_ty!(
    method_returns_self,
    "struct Foo { a: int }
     impl Foo {
         fn id(self) -> Foo { self }
     }
     let foo = Foo { a = 0 };",
    "foo.id()" => query!(Foo)
);

test_ty!(
    adt_definition_members_resolved,
    "struct Inner { v: int }
     struct Outer { inner: Inner, tag: str }
     impl Outer {
         fn make() -> Outer { Outer { inner = Inner { v = 0 }, tag = \"x\" } }
     }",
    "Outer" => adt!({
        inner: query!(Inner),
        tag: Str,
        make: func!(() -> query!(Outer))
    })
);

test_ty!(
    multiple_impl_blocks,
    "struct Foo { a: int }
     impl Foo {
         fn one(self) -> int { 1 }
     }
     impl Foo {
         fn two(self) -> int { 2 }
     }
     let foo = Foo { a = 0 };",
    "foo.one()" => Int,
    "foo.two()" => Int,
);

test_ty!(
    tuple_struct_basic,
    "struct Foo(int)
    let a = Foo(0);",
    "a.0" => Int,
);

test_ty!(
    tuple_struct_constructor_value,
    "struct Foo(int);
    let a = Foo;",
    "a" => func!((Int) -> query!(Foo))
);

test_fail!(
    use_instance_as_constructor,
    "struct Foo { x: int }
    let foo = Foo { x = 0 };
    let bar = foo { x = 1 };"
);

test_ty!(
    call_self_asso_fn_adt_struct,
    "struct Foo {}
    impl Foo {
        fn this() -> int {
            Self::that()
        }

        fn that() -> int {
            0
        }
    }",
    "Foo::this()" => Int,
);

test_ty!(
    const_on_adt_struct,
    "struct Foo {}
    impl Foo {
        const BAR = 0;
        fn fizz() -> int {
            Self::BAR
        }
    }",
    "Foo::fizz()" => Int,
);

test_ty!(
    call_self_asso_fn_tup_struct,
    "struct Foo();
    impl Foo {
        fn this() -> int {
            Self::that()
        }

        fn that() -> int {
            0
        }
    }",
    "Foo::this()" => Int,
);

test_ty!(
    const_on_tup_struct,
    "struct Foo();
    impl Foo {
        const BAR = 0;
        fn fizz() -> int {
            Self::BAR
        }
    }",
    "Foo::fizz()" => Int,
);

test_ty!(
    tup_struc_asso_fn,
    "struct Foo(int);
    impl Foo {
        fn new() -> Self {
            Self(0)
        }
    }",
    "Foo::new()" => query!(Foo),
);

test_ty!(
    bare_struct_ref,
    "struct Foo;
    let a = Foo;",
    "a" => query!(Foo)
);

test_ty!(
    dot_access_assc_const,
    "struct Foo;
    impl Foo {
        const BAR = 0;
    }
    
    let f = Foo {};",
    "f.BAR" => Int,
);

test_ty!(
    dot_access_assc_fn,
    "struct Foo;
    impl Foo {
        fn bar() -> int { 0 };
    }
    
    let f = Foo {};",
    "f.bar()" => Int,
);

test_fail!(
    double_ascc_declaration,
    "struct Foo;
    impl Foo {
        const BAR = 0;
        fn BAR() {}
    }"
);

test_fail!(
    duplicate_impl_item_fn,
    "struct S {} impl S { fn f() {} fn f() {} }",
    "struct S {} impl S { const A = 0; const A = 1; }",
);

// NotAStruct: struct-literal syntax on a non-struct value
test_fail!(not_a_struct_local_lit, "let p = 5; let q = p { x = 0 };");

// field/method access errors
test_fail!(
    field_init_wrong_type,
    "struct S { x: int } let s = S { x = \"str\" };",
    "struct S { x: int } let s = S { x = 0 }; s.x = \"str\";",
);
test_fail!(
    method_not_found_on_struct,
    "struct S {} let s = S {}; let y = s.nope();"
);
test_fail!(field_on_scalar, "let x = 5; let y = x.field;");
test_fail!(method_on_scalar, "let x = 5; let y = x.nope();");

// struct construction arity
test_fail!(
    missing_struct_fields,
    "struct S { x: int, y: int } let s = S { x = 0 };",
    "struct S { x: int, y: int } let s = S {};",
);
test_fail!(
    extra_struct_field,
    "struct S { x: int } let s = S { x = 0, z = 1 };"
);
test_fail!(expected_tuple_struct, "struct S { x: int } let s = S(0);");

// tuple indexing: out-of-bounds / non-int index / not-a-tuple receiver
test_fail!(tuple_out_of_bounds_one, "let t = (0,); let x = t.1;");
test_fail!(int_dot_index_not_tuple, "let x = 5; let y = x.0;");
test_fail!(str_dot_index_not_tuple, "let x = \"a\"; let y = x.0;");

test_fail!(
    self_struct_literal_on_tuple_struct_ices,
    "struct T(int); impl T { fn f(self) -> int { let y = Self { x = 1 }; 0 } }"
);

// regression: type errors must name user adts, not render `<adt>`
#[test]
fn mismatch_names_the_struct() {
    use super::solve_test_utils::*;
    let _t = TestResetter;
    let err = TEST_SESSION
        .with(|s| {
            s.borrow_mut().run(
                "struct Player { health: int } let p: int = Player { health = 1 };",
                "test",
            )
        })
        .unwrap_err();
    let rendered = format!("{err:?}");
    assert!(
        rendered.contains("found Player"),
        "expected the adt's name in the error: {rendered}"
    );
}
