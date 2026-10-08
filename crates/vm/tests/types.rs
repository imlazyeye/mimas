#[macro_use]
mod vm_test_utils;

use vm::Captured::*;

test_vm!(
    instance,
    "struct Foo {}",
    "Foo {}" => instance!(Foo {})
);

test_vm!(
    instance_with_fields,
    "struct Foo { a: int, b: int }",
    "Foo { a = 0, b = 1 }" => instance!(Foo { a = Int(0), b = Int(1) })
);

test_vm!(
    get_instance_field,
    "struct Foo { x: int }
    let foo = Foo { x = 42 };",
    "foo.x" => Int(42)
);

test_vm!(
    get_spilled_instance_field,
    "struct Foo { a: int, b: int, c: int, d: int, e: int }
     let foo = Foo { a = 1, b = 2, c = 3, d = 4, e = 5 };",
    "foo.e" => Int(5)
);

test_vm!(
    get_tuple_struct_field,
    "struct Wrap(int);
     let w = Wrap(7);",
    "w.0" => Int(7)
);

test_vm!(
    set_instance_field,
    "struct Foo { x: int }
    let foo = Foo { x = 42 };
    foo.x = 0;",
    "foo.x" => Int(0)
);

test_vm!(
    set_spilled_instance_field,
    "struct Foo { a: int, b: int, c: int, d: int, e: int }
     let foo = Foo { a = 1, b = 2, c = 3, d = 4, e = 5 };
     foo.e = 9;",
    "foo.e" => Int(9)
);

test_vm!(
    set_tuple_struct_field,
    "struct Wrap(float);
     let w = Wrap(1.0);
     w.0 += 2.0;",
    "w.0" => Float(3.0)
);

test_vm!(
    instance_option_field,
    "struct Foo { a: int }
    let foo: Foo? = null;",
    "foo?.a" => Null,
);

// a tuple-struct constructor is a first-class value -- bind it, then call it
test_vm!(
    tuple_struct_ctor_as_value,
    "struct Wrap(int);
     let make = Wrap;
     let w = make(7);",
    "if let Wrap(x) = w { x } else { -1 }" => Int(7),
);

test_vm!(
    const_enum_variants,
    "enum Shape {
         Dot,
         Circle(int),
         Pair(int, Shape),
     }
     struct Holder {
         shape: Shape,
     }
     const R = 3;
     const DOT = Shape::Dot;
     const CIRCLE = Shape::Circle(R + 1);
     const NESTED = Shape::Pair(1, Shape::Circle(2));
     const ALIAS = CIRCLE;
     const ALL = [Shape::Dot, CIRCLE, Shape::Circle(9), DOT];
     const PAIR = (DOT, 4);
     const HELD = Holder { shape = CIRCLE };
     const HOLDERS = [HELD, Holder { shape = Shape::Dot }];
     fn radius(shape: Shape) -> int {
         match shape {
             Shape::Dot => 0,
             Shape::Circle(r) => r,
             Shape::Pair(_, inner) => radius(inner),
         }
     }
     fn changed() -> bool {
         let all = ALL;
         all[0] = CIRCLE;
         ALL[0] != DOT
     }",
    "DOT == Shape::Dot" => Bool(true),
    "radius(CIRCLE)" => Int(4),
    "ALIAS == Shape::Circle(4)" => Bool(true),
    "radius(NESTED)" => Int(2),
    "radius(ALL[2])" => Int(9),
    "ALL[3] == DOT" => Bool(true),
    "PAIR.0 == DOT && PAIR.1 == 4" => Bool(true),
    "radius(HELD.shape)" => Int(4),
    "HOLDERS[1].shape == DOT" => Bool(true),
    "changed()" => Bool(false),
);

test_vm!(
    const_enum_variant_on_pact,
    "enum Tint {
         Red,
         Shade(int),
     }
     pact Kind {
         const TINT: Tint;
     }
     struct Bat;
     struct Rat;
     impl Kind for Bat {
         const TINT = Tint::Red;
     }
     impl Kind for Rat {
         const TINT = Tint::Shade(3);
     }
     fn tint(kind: Kind) -> Tint {
         kind.TINT
     }",
    "Bat::TINT == Tint::Red" => Bool(true),
    "tint(Rat) == Tint::Shade(3)" => Bool(true),
    "Tint::Shade(3) in Kind::*::TINT" => Bool(true),
);

test_vm!(
    const_array_of_const_arithmetic,
    "const R = 4;
     const HALVES = [R ~/ 2, R, R * 2];",
    "HALVES[0] + HALVES[2]" => Int(10),
);

test_vm!(
    const_structs_and_optional_members,
    "enum Shape {
         Dot,
         Maybe(Shape?),
     }
     struct Wrap(int);
     struct Bat;
     struct Holder {
         shape: Shape,
         wrap: Wrap,
     }
     const R = 2;
     const W = Wrap(R + 1);
     const B = Bat;
     const NONE = Shape::Maybe(null);
     const SOME = Shape::Maybe(Shape::Dot);
     const HELD = Holder { shape = SOME, wrap = W };
     fn changed() -> bool {
         let held = HELD;
         held.wrap = Wrap(0);
         HELD.wrap != W
     }",
    "W == Wrap(3)" => Bool(true),
    "B == Bat" => Bool(true),
    "NONE != SOME" => Bool(true),
    "HELD == HELD" => Bool(true),
    "HELD.shape == Shape::Maybe(Shape::Dot)" => Bool(true),
    "changed()" => Bool(false),
);

test_vm!(
    const_collection_equality_folds,
    "const X = 1;
     const SAME = [X] == [1];
     const DIFF = (X + 1, \"a\") != (2, \"a\");
     const NESTED = [[X], [2]] == [[1], [X + 1]];",
    "SAME" => Bool(true),
    "DIFF" => Bool(false),
    "NESTED" => Bool(true),
);

test_vm!(
    forward_const_arithmetic,
    "const A: int = B + 1;
     const B = C + 1;
     const C = 1;
     fn local() -> int {
         let value = X;
         const X = Y + 1;
         const Y = 2;
         value
     }",
    "A" => Int(3),
    "local()" => Int(3),
);

test_vm!(
    constant_semantics,
    "const LT = 9007199254740992 < 9007199254740993;
     const AND = false && (1 ~/ 0 == 1);
     const OR = true || (1 ~/ 0 == 1);
     const FLOAT = +-1.5;
     const DIV = 1.0 / 0.0;
     const REM = 5.5 % 2.0;
     const IDIV = 6.5 ~/ 2.0;",
    "LT" => Bool(true),
    "AND" => Bool(false),
    "OR" => Bool(true),
    "FLOAT" => Float(-1.5),
    "DIV" => Float(f64::INFINITY),
    "REM" => Float(1.5),
    "IDIV" => Float(3.0),
);
