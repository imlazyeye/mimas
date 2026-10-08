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

test_vm!(
    const_nan_equality,
    "struct Sample { value: float }
     const NAN = 0 / 0;
     const TABLE = [NAN];
     const GRID = [[NAN]];
     const LOOKUP = ~{ value = NAN };
     const SAMPLE = Sample { value = NAN };
     let table = TABLE;
     let grid = [table];
     let lookup = LOOKUP;
     let sample = SAMPLE;",
    "TABLE == TABLE" => Bool(false),
    "TABLE != TABLE" => Bool(true),
    "GRID == GRID" => Bool(false),
    "GRID[0] == GRID[0]" => Bool(false),
    "LOOKUP == LOOKUP" => Bool(false),
    "SAMPLE == SAMPLE" => Bool(false),
    "table == table" => Bool(true),
    "table != table" => Bool(false),
    "grid[0] == table" => Bool(true),
    "lookup == lookup" => Bool(true),
    "sample == sample" => Bool(true),
);

test_vm!(
    const_data_preserves_layout,
    r#"struct Point {
         x: int,
         y: int,
     }
     enum Shape {
         Named { point: Point, label: str },
     }
     const OFFSET = 2;
     const ORIGIN = Point { y = OFFSET * 3, x = 0x10 + OFFSET };
     const NAMED = Shape::Named { label = "ab" + "cd", point = ORIGIN };
     const TABLE = ~{
         first = ORIGIN,
         second = Point { y = 7, x = 8 },
     };
     const ALIAS = TABLE;"#,
    "ORIGIN.x" => Int(18),
    "ORIGIN.y" => Int(6),
    r#"match NAMED {
         Shape::Named { point, label } => label == "abcd" && point.x == 18,
     }"# => Bool(true),
    r#"ALIAS["second"]!.x"# => Int(8),
    r#"ALIAS["missing"]"# => Null,
);

test_vm!(
    const_inspection,
    r#"const TABLE = [10, 20, 30];
     const DIRS = [(0, 1), (1, 0)];
     const GRID = [[1, 2], [3, 4]];
     const NAMES = ~{ a = 1, b = 2 };
     fn row_sum() -> int {
         let sum = 0;
         for value in GRID[1] {
             sum += value;
         }
         sum
     }
     fn values() -> int {
         let sum = 0;
         for (key, value) in NAMES {
             sum += value;
         }
         sum
     }"#,
    "(TABLE)[1]" => Int(20),
    "DIRS[1].0" => Int(1),
    "GRID[1][0]" => Int(3),
    "row_sum()" => Int(7),
    "values()" => Int(3),
    "20 in TABLE" => Bool(true),
    "25 !in TABLE" => Bool(true),
    r#""a" in NAMES"# => Bool(true),
    "TABLE == [10, 20, 30]" => Bool(true),
    "[10, 20] != TABLE" => Bool(true),
    "NAMES == ~{ b = 2, a = 1 }" => Bool(true),
);

test_vm!(
    const_part_is_its_own_value,
    r#"struct Point { x: int }
     struct Line { to: Point }
     enum Shape {
         Dot,
         Box(Point),
     }
     const GRID = [[1, 2], [3, 4]];
     const POINTS = [Point { x = 1 }, Point { x = 3 }];
     const LINE = Line {
         to = Point { x = 5 },
     };
     const SHAPES = [Shape::Dot, Shape::Box(Point { x = 7 })];
     const LOOKUP = ~{ evens = [2, 4], odds = [1, 3] };
     fn first(row: [int]) -> int {
         row[0] = 9;
         row[0]
     }
     fn corner(shape: Shape) -> int {
         match shape {
             Shape::Dot => 0,
             Shape::Box(point) => {
                 point.x += 1;
                 point.x
             }
         }
     }
     fn bound() -> int {
         let grid = GRID;
         grid[0][0] = 9;
         GRID[0][0]
     }
     fn element() -> int {
         let point = POINTS[0];
         point.x = 9;
         POINTS[0].x
     }
     fn field() -> int {
         let to = LINE.to;
         to.x = 9;
         LINE.to.x
     }
     fn looped() -> int {
         for point in POINTS {
             point.x = 9;
         }
         POINTS[1].x
     }
     fn pairs() -> int {
         for (name, list) in LOOKUP {
             list[0] = 9;
         }
         LOOKUP["evens"]![0]
     }
     fn passed() -> int {
         first(GRID[0]) + GRID[0][0]
     }
     fn matched() -> int {
         corner(SHAPES[1]) + corner(SHAPES[1])
     }"#,
    "bound()" => Int(1),
    "element()" => Int(1),
    "field()" => Int(5),
    "looped()" => Int(3),
    "pairs()" => Int(2),
    "passed()" => Int(10),
    "matched()" => Int(16),
);

test_vm!(
    fieldless_variant_eq,
    "enum Foo {
         Bar,
         Baz,
     }
     const BAR = Foo::Bar;
     const ALIAS = (BAR);
     let bar = Foo::Bar;
     let baz = Foo::Baz;",
    "bar == Foo::Bar" => Bool(true),
    "Foo::Bar == baz" => Bool(false),
    "bar != Foo::Bar" => Bool(false),
    "Foo::Bar != baz" => Bool(true),
    "bar == BAR" => Bool(true),
    "BAR != bar" => Bool(false),
    "ALIAS == bar" => Bool(true),
    "Foo::Baz != BAR" => Bool(true),
);

test_vm!(
    fieldless_struct_eq,
    "struct Cat;
     const CAT = Cat;
     let cat = Cat;
     let absent: Cat? = null;",
    "cat == Cat" => Bool(true),
    "Cat != cat" => Bool(false),
    "absent == CAT" => Bool(false),
    "CAT != absent" => Bool(true),
);

test_vm!(
    fieldless_eq_on_option,
    "enum Foo {
         Bar,
         Baz,
     }
     const BAR = Foo::Bar;
     let present: Foo? = Foo::Bar;
     let absent: Foo? = null;",
    "present == Foo::Bar" => Bool(true),
    "Foo::Baz != present" => Bool(true),
    "absent == BAR" => Bool(false),
    "BAR != absent" => Bool(true),
);

test_vm!(
    fieldless_eq_on_pact_value,
    "pact Pet {
         fn legs(self) -> int;
     }
     struct Cat;
     enum Bird {
         Owl,
         Wren,
     }
     impl Pet for Cat {
         fn legs(self) -> int {
             4
         }
     }
     impl Pet for Bird {
         fn legs(self) -> int {
             2
         }
     }
     const OWL = Bird::Owl;
     let pets = [Cat, Bird::Owl, Bird::Wren];",
    "pets[0] == Cat" => Bool(true),
    "pets[1] == Cat" => Bool(false),
    "pets[1] != Cat" => Bool(true),
    "pets[1] == Bird::Owl" => Bool(true),
    "pets[2] == OWL" => Bool(false),
    "pets[0] != OWL" => Bool(true),
);

test_vm!(
    fieldless_eq_runs_other_side_once,
    "enum Foo {
         Bar,
         Baz,
     }
     fn next(calls: [int]) -> Foo {
         calls[0] += 1;
         Foo::Baz
     }
     let calls = [0];
     let same = next(calls) == Foo::Bar;
     let differs = Foo::Bar != next(calls);",
    "same" => Bool(false),
    "differs" => Bool(true),
    "calls[0]" => Int(2),
);
