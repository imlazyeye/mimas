use std::collections::HashMap;

use vm::{Captured, Ctx, MimasEnum, MimasStruct, Ty, Vm, conversion::Raisable};

#[track_caller]
fn run_with(installer: impl FnOnce(&mut vm::api::Api<'_, '_>), source: &str) -> Captured {
    let mut vm = Vm::execute(source, installer).expect("test source compiled and ran");
    vm.resolve_name("TEST_VALUE")
        .expect("TEST_VALUE was bound by the test source")
}

// each arm: `name: "source" => Expected`; source is a full program binding TEST_VALUE.
// installer is the shared native-install fn for the file.
macro_rules! native_tests {
    ($installer:path; $( $(#[$attr:meta])* $name:ident: $src:expr => $expected:expr );+ $(;)? ) => {
        $(
            #[test]
            $(#[$attr])*
            fn $name() {
                pretty_assertions::assert_eq!(run_with($installer, $src), $expected);
            }
        )+
    };
}

fn echo_int(_ctx: Ctx<'_>, n: i64) -> i64 {
    n + 1
}
fn echo_float(_ctx: Ctx<'_>, f: f64) -> f64 {
    f * 2.0
}
fn echo_bool(_ctx: Ctx<'_>, b: bool) -> bool {
    !b
}
fn echo_str(_ctx: Ctx<'_>, s: String) -> String {
    format!("{s}!")
}
fn returns_unit(_ctx: Ctx<'_>) {}

fn echo_u8(_ctx: Ctx<'_>, n: u8) -> u8 {
    n
}
fn echo_i32(_ctx: Ctx<'_>, n: i32) -> i32 {
    n
}
fn echo_f32(_ctx: Ctx<'_>, f: f32) -> f32 {
    f
}

fn maybe_int(_ctx: Ctx<'_>, present: bool) -> Option<i64> {
    if present { Some(7) } else { None }
}
fn unwrap_or_zero(_ctx: Ctx<'_>, v: Option<i64>) -> i64 {
    v.unwrap_or(0)
}
fn maybe_str(_ctx: Ctx<'_>, present: bool) -> Option<String> {
    if present { Some("hi".into()) } else { None }
}

fn make_pair(_ctx: Ctx<'_>) -> (i64, String) {
    (3, "x".into())
}
fn make_triple(_ctx: Ctx<'_>) -> (i64, bool, f64) {
    (1, true, 2.5)
}
fn sum_pair(_ctx: Ctx<'_>, p: (i64, i64)) -> i64 {
    p.0 + p.1
}

fn make_ints(_ctx: Ctx<'_>) -> Vec<i64> {
    vec![1, 2, 3]
}
fn make_strs(_ctx: Ctx<'_>) -> Vec<String> {
    vec!["a".into(), "b".into()]
}
fn sum_vec(_ctx: Ctx<'_>, xs: Vec<i64>) -> i64 {
    xs.into_iter().sum()
}
fn make_nested(_ctx: Ctx<'_>) -> Vec<Vec<i64>> {
    vec![vec![1], vec![2, 3]]
}

fn make_map(_ctx: Ctx<'_>) -> HashMap<String, i64> {
    let mut m = HashMap::new();
    m.insert("k".into(), 9);
    m
}
fn lookup(_ctx: Ctx<'_>, m: HashMap<String, i64>, key: String) -> i64 {
    *m.get(&key).unwrap_or(&-1)
}

fn checked_div(_ctx: Ctx<'_>, a: i64, b: i64) -> Raisable<i64> {
    if b == 0 {
        Raisable::Raised("divide by zero".into())
    } else {
        Raisable::Ok(a / b)
    }
}
fn parse_int(_ctx: Ctx<'_>, s: String) -> Raisable<i64> {
    s.parse::<i64>().map_err(|e| e.to_string()).into()
}

#[derive(MimasStruct, Clone, PartialEq, Debug)]
struct Vec2 {
    x: f64,
    y: f64,
}
#[derive(MimasStruct, Clone, PartialEq, Debug)]
struct Pair(i64, i64);
#[derive(MimasStruct, Clone, PartialEq, Debug)]
struct Tag;

#[derive(MimasEnum, Clone, PartialEq, Debug)]
enum Dir {
    North,
    South,
    Custom(i64),
    Vec { dx: i64, dy: i64 },
}

#[derive(MimasEnum, Clone, PartialEq, Debug)]
enum Tree {
    Leaf(i64),
    Branch(Vec<Tree>),
}

fn origin(_ctx: Ctx<'_>) -> Vec2 {
    Vec2 { x: 0.0, y: 0.0 }
}
fn vec2_len2(_ctx: Ctx<'_>, v: Vec2) -> f64 {
    v.x * v.x + v.y * v.y
}
fn make_pair_struct(_ctx: Ctx<'_>) -> Pair {
    Pair(4, 5)
}
fn pair_sum(_ctx: Ctx<'_>, p: Pair) -> i64 {
    p.0 + p.1
}
fn tag(_ctx: Ctx<'_>) -> Tag {
    Tag
}
fn is_tag(_ctx: Ctx<'_>, _t: Tag) -> bool {
    true
}
fn make_north(_ctx: Ctx<'_>) -> Dir {
    Dir::North
}
fn describe_dir(_ctx: Ctx<'_>, d: Dir) -> String {
    match d {
        Dir::North => "N".into(),
        Dir::South => "S".into(),
        Dir::Custom(n) => format!("C{n}"),
        Dir::Vec { dx, dy } => format!("V{dx},{dy}"),
    }
}

fn sample_tree(_ctx: Ctx<'_>) -> Tree {
    Tree::Branch(vec![Tree::Leaf(1), Tree::Branch(vec![Tree::Leaf(2)])])
}
fn sum_tree(_ctx: Ctx<'_>, t: Tree) -> i64 {
    match t {
        Tree::Leaf(n) => n,
        Tree::Branch(kids) => kids.into_iter().map(|k| sum_tree(_ctx, k)).sum(),
    }
}

fn doubled(_ctx: Ctx<'_>, v: Vec2) -> Vec2 {
    Vec2 {
        x: v.x * 2.0,
        y: v.y * 2.0,
    }
}
fn splat(_ctx: Ctx<'_>, n: f64) -> Vec2 {
    Vec2 { x: n, y: n }
}

fn install_all(api: &mut vm::api::Api<'_, '_>) {
    api.add_adt::<Vec2>();
    api.add_adt::<Pair>();
    api.add_adt::<Tag>();
    api.add_adt::<Dir>();
    api.add_adt::<Tree>();

    api.add(echo_int);
    api.add(echo_float);
    api.add(echo_bool);
    api.add(echo_str);
    api.add(returns_unit);
    api.add(echo_u8);
    api.add(echo_i32);
    api.add(echo_f32);
    api.add(maybe_int);
    api.add(unwrap_or_zero);
    api.add(maybe_str);
    api.add(make_pair);
    api.add(make_triple);
    api.add(sum_pair);
    api.add(make_ints);
    api.add(make_strs);
    api.add(sum_vec);
    api.add(make_nested);
    api.add(make_map);
    api.add(lookup);
    api.add(checked_div);
    api.add(parse_int);

    api.add(origin);
    api.add(vec2_len2);
    api.add(make_pair_struct);
    api.add(pair_sum);
    api.add(tag);
    api.add(is_tag);
    api.add(make_north);
    api.add(describe_dir);
    api.add(sample_tree);
    api.add(sum_tree);

    // receiver method on a derived-struct type, inferred from first non-ctx param
    api.add_method(doubled);
    // associated fn on the float builtin namespace
    api.add_assoc(Ty::Float, splat);
}

native_tests! { install_all;
    native_int_in_out: "let TEST_VALUE = echo_int(5);" => Captured::Int(6);
    native_float_in_out: "let TEST_VALUE = echo_float(2.5);" => Captured::Float(5.0);
    native_bool_in_out: "let TEST_VALUE = echo_bool(true);" => Captured::Bool(false);
    native_str_in_out: r#"let TEST_VALUE = echo_str("hi");"# => Captured::Str("hi!".into());
    native_unit_return_is_null: "let TEST_VALUE = returns_unit();" => Captured::Null;
    native_u8_roundtrips: "let TEST_VALUE = echo_u8(200);" => Captured::Int(200);
    native_i32_roundtrips: "let TEST_VALUE = echo_i32(123);" => Captured::Int(123);
    native_f32_roundtrips: "let TEST_VALUE = echo_f32(1.5);" => Captured::Float(1.5);

    native_option_some: "let TEST_VALUE = maybe_int(true);" => Captured::Int(7);
    native_option_none_is_null: "let TEST_VALUE = maybe_int(false);" => Captured::Null;
    native_option_str_some: "let TEST_VALUE = maybe_str(true);" => Captured::Str("hi".into());
    native_takes_option_null: "let v: int? = null;\nlet TEST_VALUE = unwrap_or_zero(v);" => Captured::Int(0);
    native_takes_option_some: "let v: int? = 11;\nlet TEST_VALUE = unwrap_or_zero(v);" => Captured::Int(11);

    native_returns_pair: "let TEST_VALUE = make_pair();"
        => Captured::Array(vec![Captured::Int(3), Captured::Str("x".into())]);
    native_returns_triple: "let TEST_VALUE = make_triple();"
        => Captured::Array(vec![Captured::Int(1), Captured::Bool(true), Captured::Float(2.5)]);
    native_takes_tuple: "let TEST_VALUE = sum_pair((4, 6));" => Captured::Int(10);

    native_returns_int_vec: "let TEST_VALUE = make_ints();"
        => Captured::Array(vec![Captured::Int(1), Captured::Int(2), Captured::Int(3)]);
    native_returns_str_vec: "let TEST_VALUE = make_strs();"
        => Captured::Array(vec![Captured::Str("a".into()), Captured::Str("b".into())]);
    native_takes_vec: "let TEST_VALUE = sum_vec([10, 20, 30]);" => Captured::Int(60);
    native_takes_empty_vec: "let TEST_VALUE = sum_vec([]);" => Captured::Int(0);
    native_returns_nested_vec: "let TEST_VALUE = make_nested();"
        => Captured::Array(vec![
            Captured::Array(vec![Captured::Int(1)]),
            Captured::Array(vec![Captured::Int(2), Captured::Int(3)]),
        ]);

    native_returns_dict: "let TEST_VALUE = make_map();"
        => Captured::Dict(vec![("k".into(), Captured::Int(9))]);
    native_takes_dict: r#"let TEST_VALUE = lookup(~{ a = 1, b = 2 }, "b");"# => Captured::Int(2);
    native_takes_dict_missing_key: r#"let TEST_VALUE = lookup(~{ a = 1 }, "z");"# => Captured::Int(-1);

    native_raisable_ok_unwrap: "let TEST_VALUE = checked_div(10, 2)!;" => Captured::Int(5);
    native_raisable_raised_absolve: "let TEST_VALUE = checked_div(1, 0) absolve |e| -99;" => Captured::Int(-99);
    native_raisable_parse_ok: r#"let TEST_VALUE = parse_int("42")!;"# => Captured::Int(42);
    native_raisable_parse_raised: r#"let TEST_VALUE = parse_int("nope") absolve |e| 0;"# => Captured::Int(0);

    native_returns_named_struct: "let TEST_VALUE = vec2_len2(origin());" => Captured::Float(0.0);
    native_constructs_named_struct_from_mimas:
        "let TEST_VALUE = vec2_len2(Vec2 { x = 3.0, y = 4.0 });" => Captured::Float(25.0);
    native_returns_tuple_struct: "let TEST_VALUE = pair_sum(make_pair_struct());" => Captured::Int(9);
    native_constructs_tuple_struct_from_mimas: "let TEST_VALUE = pair_sum(Pair(7, 8));" => Captured::Int(15);
    native_unit_struct_roundtrip: "let TEST_VALUE = is_tag(tag());" => Captured::Bool(true);

    native_returns_enum_unit_variant: "let TEST_VALUE = describe_dir(make_north());" => Captured::Str("N".into());
    native_constructs_enum_unit_variant: "let TEST_VALUE = describe_dir(Dir::South);" => Captured::Str("S".into());
    native_constructs_enum_tuple_variant: "let TEST_VALUE = describe_dir(Dir::Custom(5));" => Captured::Str("C5".into());
    native_constructs_enum_named_variant:
        "let TEST_VALUE = describe_dir(Dir::Vec { dx = 1, dy = 2 });" => Captured::Str("V1,2".into());
    native_enum_matched_in_mimas:
        "let TEST_VALUE = match Dir::Custom(9) { Dir::Custom(n) => n, _ => 0 };" => Captured::Int(9);

    // recursive enum (Vec<Tree>) registers and roundtrips through a native
    native_recursive_enum_roundtrip: "let TEST_VALUE = sum_tree(sample_tree());" => Captured::Int(3);

    native_method_on_struct_receiver:
        "let TEST_VALUE = vec2_len2(Vec2 { x = 1.0, y = 0.0 }.doubled());" => Captured::Float(4.0);
    native_assoc_fn_on_float: "let TEST_VALUE = vec2_len2(float::splat(2.0));" => Captured::Float(8.0);
}

fn opt_tail(_ctx: Ctx<'_>, a: i64, b: Option<i64>) -> i64 {
    a + b.unwrap_or(100)
}
fn opt_mid(_ctx: Ctx<'_>, a: Option<i64>, b: i64) -> i64 {
    a.unwrap_or(0) + b
}

fn install_optional(api: &mut vm::api::Api<'_, '_>) {
    api.add(opt_tail);
    api.add(opt_mid);
}

native_tests! { install_optional;
    native_trailing_option_provided: "let TEST_VALUE = opt_tail(1, 2);" => Captured::Int(3);
    native_trailing_option_omitted: "let TEST_VALUE = opt_tail(1);" => Captured::Int(101);
    native_trailing_option_null: "let TEST_VALUE = opt_tail(1, null);" => Captured::Int(101)
}

// a non-trailing Option param is NOT omittable -- positional args can't skip a slot
#[test]
fn native_mid_option_still_required() {
    assert!(
        Vm::compile("let TEST_VALUE = opt_mid(5);", install_optional).is_err(),
        "omitting a non-trailing Option param should be an arity error",
    );
}

fn install_const(api: &mut vm::api::Api<'_, '_>) {
    api.constant("ANSWER", Ty::Int, shared::Literal::Int(42), "");
}

native_tests! { install_const;
    native_prelude_constant: "let TEST_VALUE = ANSWER;" => Captured::Int(42);
    native_constant_in_expr: "let TEST_VALUE = ANSWER + 8;" => Captured::Int(50)
}

fn mod_double(_ctx: Ctx<'_>, n: i64) -> i64 {
    n * 2
}

#[derive(MimasStruct, Clone, PartialEq, Debug)]
struct Widget {
    id: i64,
}

fn make_widget(_ctx: Ctx<'_>, id: i64) -> Widget {
    Widget { id }
}
fn widget_id(_ctx: Ctx<'_>, w: Widget) -> i64 {
    w.id
}

fn install_module(api: &mut vm::api::Api<'_, '_>) {
    {
        let mut m = api.module("gadget");
        m.add_adt::<Widget>();
        m.add(mod_double);
        m.add(make_widget);
        m.constant("VERSION", Ty::Int, shared::Literal::Int(1), "");
    }
    api.add(widget_id);
}

native_tests! { install_module;
    native_module_fn_via_path: "let TEST_VALUE = gadget::mod_double(21);" => Captured::Int(42);
    native_module_constant_via_path: "let TEST_VALUE = gadget::VERSION;" => Captured::Int(1);
    native_module_adt_construct_and_consume:
        "let TEST_VALUE = widget_id(gadget::Widget { id = 99 });" => Captured::Int(99);
    native_module_adt_via_native_ctor:
        "let TEST_VALUE = widget_id(gadget::make_widget(7));" => Captured::Int(7)
}

#[derive(MimasEnum, Clone, PartialEq, Debug)]
enum Shape {
    Circle(f64),
    Rect { w: f64, h: f64 },
}

fn unit_circle(_ctx: Ctx<'_>) -> Shape {
    Shape::Circle(1.0)
}
fn describe_shape(_ctx: Ctx<'_>, s: Shape) -> String {
    match s {
        Shape::Circle(r) => format!("circle:{r}"),
        Shape::Rect { w, h } => format!("rect:{w}:{h}"),
    }
}

fn install_geo(api: &mut vm::api::Api<'_, '_>) {
    api.module("geo").add_adt::<Shape>();
    api.add(unit_circle);
    api.add(describe_shape);
}

native_tests! { install_geo;
    module_enum_native_roundtrip: "let TEST_VALUE = describe_shape(unit_circle());" => Captured::Str("circle:1".into());
    module_enum_construct_via_path:
        "let TEST_VALUE = describe_shape(geo::Shape::Circle(2.0));" => Captured::Str("circle:2".into());
    module_enum_named_variant_via_path:
        "let TEST_VALUE = describe_shape(geo::Shape::Rect { w = 3.0, h = 4.0 });" => Captured::Str("rect:3:4".into());
    module_enum_use_then_construct:
        "use geo::Shape; let TEST_VALUE = describe_shape(Shape::Circle(5.0));" => Captured::Str("circle:5".into());
    module_enum_type_annotation:
        "let s: geo::Shape = geo::Shape::Circle(7.0); let TEST_VALUE = describe_shape(s);" => Captured::Str("circle:7".into());
    module_enum_pattern_match:
        "let TEST_VALUE = match geo::Shape::Circle(9.0) { geo::Shape::Circle(r) => r, _ => 0.0 };" => Captured::Float(9.0);
    module_enum_array_annotation:
        "let xs: [geo::Shape] = [geo::Shape::Circle(8.0)]; let TEST_VALUE = describe_shape(xs[0]);" => Captured::Str("circle:8".into())
}
