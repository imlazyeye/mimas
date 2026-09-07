use crate::components::Ty::*;

test_multi_file!(
    basic_module,
    foo => "module foo; pub const BAR: int = 0;";
    "foo::BAR" => Int,
);

test_multi_file!(
    identity_module,
    foo => "module @; pub const BAR: int = 0;";
    "foo::BAR" => Int,
);

test_multi_file!(
    two_modules,
    foo => "module @; pub const BAR: int = 0;",
    fizz => "module @; use foo; pub fn buzz() -> int { foo::BAR }";
    "foo::BAR" => Int,
    "fizz::buzz" => func!(() -> Int),
);

test_multi_file!(
    module_with_function,
    foo => "module @; pub fn bar() -> int { 0 }";
    "foo::bar()" => Int,
);

test_multi_file!(
    module_with_struct,
    foo => "module @; pub struct Bar { x: int }";
    "foo::Bar" => adt!({ x: Int }),
);

test_multi_file!(
    direct_module_path_without_import,
    foo => "module @; pub const BAR: int = 0;",
    fizz => "module @; pub fn buzz() -> int { foo::BAR }";
    "fizz::buzz()" => Int,
);

test_multi_file!(
    singular_import_visible_in_function,
    foo => "module @; pub const BAR: int = 0;",
    fizz => "module @; use foo::BAR; pub fn buzz() -> int { BAR }";
    "fizz::buzz()" => Int,
);

test_multi_file!(
    #[should_panic]
    singular_import_not_reexported,
    foo => "module @; pub const BAR: int = 0;",
    fizz => "module @; use foo::BAR; pub fn buzz() -> int { BAR }";
    "fizz::BAR" => Int,
);

test_multi_file!(
    multi_import_visible_in_function,
    foo => "module @; pub const BAR: int = 0; pub const BAZ: str = \"\";",
    fizz => "module @; use foo::{ BAR, BAZ }; pub fn bar() -> int { BAR } pub fn baz() -> str { BAZ }";
    "fizz::bar()" => Int,
    "fizz::baz()" => Str,
);

test_multi_file!(
    #[should_panic]
    multi_import_not_reexported,
    foo => "module @; pub const BAR: int = 0; pub const BAZ: str = \"\";",
    fizz => "module @; use foo::{ BAR, BAZ }; pub fn bar() -> int { BAR }";
    "fizz::BAR" => Int,
);

test_multi_file!(
    glob_import_visible_in_function,
    foo => "module @; pub const BAR: int = 0; pub const BAZ: str = \"\";",
    fizz => "module @; use foo::*; pub fn bar() -> int { BAR } pub fn baz() -> str { BAZ }";
    "fizz::bar()" => Int,
    "fizz::baz()" => Str,
);

test_multi_file!(
    #[should_panic]
    glob_import_not_reexported,
    foo => "module @; pub const BAR: int = 0;",
    fizz => "module @; use foo::*; pub fn bar() -> int { BAR }";
    "fizz::BAR" => Int,
);

test_multi_file!(
    used_type_in_fn_signature,
    foo => "module @; pub struct Bar { x: int }",
    fizz => "module @; use foo::Bar; pub fn take(b: Bar) -> int { b.x }";
    "fizz::take" => func!((adt!({ x: Int })) -> Int),
);

test_multi_file!(
    modules_are_hoisted_before_solving,
    fizz => "module @; use foo; pub fn buzz() -> int { foo::BAR }",
    foo => "module @; pub const BAR: int = 0;";
    "fizz::buzz()" => Int,
);

test_multi_file!(
    circular_module_functions,
    foo => "module @; use fizz; pub fn foo() -> int { fizz::bar() } pub fn value() -> int { 1 }",
    fizz => "module @; use foo; pub fn bar() -> int { foo::value() }";
    "foo::foo()" => Int,
    "fizz::bar()" => Int,
);

test_multi_file_fail!(
    private_const_cross_module,
    foo => "module @; const SECRET: int = 0;",
    main => "let x = foo::SECRET;";
);

test_multi_file_fail!(
    private_fn_cross_module,
    foo => "module @; fn helper() -> int { 0 }",
    main => "let x = foo::helper();";
);

test_multi_file_fail!(
    private_struct_cross_module,
    foo => "module @; struct Bar { x: int }",
    main => "let b = foo::Bar { x = 0 };";
);

test_multi_file_fail!(
    private_via_use_still_errors,
    foo => "module @; const SECRET: int = 0;",
    main => "use foo::SECRET; let x = SECRET;";
);

test_multi_file_fail!(
    private_method_cross_module,
    foo => "module @; pub struct Bar { x: int } impl Bar { fn secret(self) -> int { 0 } }",
    main => "let b = foo::Bar { x = 0 }; let v = b.secret();";
);

test_multi_file!(
    pub_method_cross_module,
    foo => "module @; pub struct Bar { x: int } impl Bar { pub fn ok(self) -> int { 0 } }",
    main => "module @; pub fn use_it() -> int { foo::Bar { x = 0 }.ok() }";
    "main::use_it" => func!(() -> Int),
);

test_multi_file!(
    private_const_in_same_module_ok,
    foo => "module @; const SECRET: int = 0; pub fn get() -> int { SECRET }";
    "foo::get" => func!(() -> Int),
);

// the exhaustion diagnostic looks up the missing variant's def site across files; guards that path.
test_multi_file_fail!(
    match_imported_enum_non_exhaustive,
    foo => "module @; pub enum Color { Red, Blue, Green }",
    fizz => "module @; use foo::Color; pub fn pick(c: Color) -> int { match c { Color::Red => 0, Color::Blue => 1 } }";
);
