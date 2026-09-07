use crate::components::Ty::*;

test_ty!(
    r#enum,
    "enum Foo {}",
    "Foo" => enu!({})
);

test_ty!(
    enum_with_fields,
    "enum Foo {
        A,
        B,
    }",
    "Foo" => enu!({
        enum_member!(A),
        enum_member!(B),
    })
);

test_ty!(
    enum_with_adt_fields,
    "enum Foo {
        A {
            x: int,
        },
        B {
            y: int
        },
    }",
    "Foo" => enu!({
        enum_member!(A {
            x: Int
        }),
        enum_member!(B {
            y: Int
        }),
    })
);

test_ty!(
    enum_with_tuple_fields,
    "enum Foo {
        A(),
        B(int),
    }",
    "Foo" => enu!({
        enum_member!(A()),
        enum_member!(B(Int)),
    })
);

test_ty!(
    enum_adt_literal,
    "enum Foo { Bar }
    let a: Foo = Foo::Bar {};",
    "Foo" => enu!({ enum_member!(Bar) }),
    "a" => query!(Foo)
);

test_ty!(
    match_enums,
    "enum Foo { A, B }
    let foo = Foo::A {};
    let bar = match foo {
        Foo::A => 0,
        Foo::B => 1
    };",
    "bar" => Int
);

test_ty!(
    enum_tuple_literal,
    "enum Foo { Bar(int) }
    let a: Foo = Foo::Bar(0);",
    "Foo" => enu!({ enum_member!(Bar(Int)) }),
    "a" => query!(Foo)
);

test_ty!(
    impl_enum_function,
    "enum Foo {};
    impl Foo {
        fn bar() {}
    }",
    "Foo::bar()" => Unit,
);

test_ty!(
    enum_mixed_variants,
    "enum Color {
        Red,
        Rgb(int, int, int),
        Named { name: str }
    }",
    "Color" => enu!({
        enum_member!(Red),
        enum_member!(Rgb(Int, Int, Int)),
        enum_member!(Named { name: Str })
    })
);

test_ty!(
    enum_const_in_impl,
    "enum Color { Red }
     impl Color {
         const COUNT = 1;
     }",
    "Color::COUNT" => Int
);

test_ty!(
    self_ty_ref_in_enum,
    "enum Foo { Bar }
    impl Foo {
        fn new() -> Self {
            Self::Bar
        }
    }",
    "Foo::new()" => query!(Foo),
);

test_ty!(
    call_self_asso_fn_enum,
    "enum Foo {}
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
    bare_enum_ref,
    "enum Foo { Bar }
    let a = Foo::Bar;",
    "a" => query!(Foo)
);

test_ty!(
    or_pattern_shared_binding,
    "enum Foo { Bar(int), Fizz(int) }
    let r = match Foo::Bar(0) {
        Foo::Bar(n) | Foo::Fizz(n) => n + 1,
    };",
    "r" => Int
);

test_ty!(
    or_pattern_fieldless,
    "enum Color { Red, Green, Blue }
    let r = match Color::Red {
        Color::Red | Color::Green => 0,
        Color::Blue => 1,
    };",
    "r" => Int
);

test_ty!(
    or_pattern_struct_variants_shared_field,
    "enum E { A { x: int }, B { x: int } }
    let v = E::A { x = 0 };
    let r = match v {
        E::A { x } | E::B { x } => x + 1,
    };",
    "r" => Int
);

test_fail!(
    or_pattern_payload_type_mismatch,
    "enum Foo { Bar(int), Buzz(str) }
    let r = match Foo::Bar(0) {
        Foo::Bar(n) | Foo::Buzz(n) => 0,
    };"
);

test_fail!(
    or_pattern_inconsistent_names,
    "enum Foo { Bar(int), Fizz(int) }
    let r = match Foo::Bar(0) {
        Foo::Bar(n) | Foo::Fizz(m) => 0,
    };"
);

test_fail!(
    or_pattern_some_none,
    "enum Opt { Some(int), Nope }
    let r = match Opt::Nope {
        Opt::Some(x) | Opt::Nope => 0,
    };"
);

// NotAnEnum: enum-path syntax on a struct
test_fail!(
    not_an_enum_path_on_struct,
    "struct S { x: int } let s = S::Foo {};"
);

// EnumNotConstructable / ExpectedTupleStruct
test_fail!(enum_constructed_with_parens, "enum E { A } let e = E();");
test_fail!(
    enum_constructed_with_struct_literal,
    "enum E { A } let e = E {};"
);

// VariantNotFound: struct-literal and tuple-call forms
test_fail!(
    variant_not_found,
    "enum E { A } let e = E::B {};",
    "enum E { A(int) } let e = E::Z(0);",
);
