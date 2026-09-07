#[macro_use]
mod vm_test_utils;

use vm::Captured::*;

test_vm!(
    module_const_access,
    files {
        foo => "module @; pub const BAR: int = 7;",
        main => "let TEST_VALUE = foo::BAR;",
    } => Int(7),
);

test_vm!(
    module_use_const_access,
    files {
        foo => "module @; pub const BAR: int = 7;",
        main => "use foo::BAR; let TEST_VALUE = BAR;",
    } => Int(7),
);

test_vm!(
    module_function_call,
    files {
        foo => "module @; pub fn answer() -> int { 7 }",
        main => "let TEST_VALUE = foo::answer();",
    } => Int(7),
);

test_vm!(
    module_struct_instance_access,
    files {
        foo => "module @; pub struct Bar { x: int }",
        main => "use foo::Bar; let a = Bar { x = 7 }; let TEST_VALUE = a.x;",
    } => Int(7),
);

test_vm!(
    module_direct_struct_instance_access,
    files {
        foo => "module @; pub struct Bar { x: int }",
        main => "let a = foo::Bar { x = 7 }; let TEST_VALUE = a.x;",
    } => Int(7),
);

test_vm!(
    module_tuple_struct_instance_access,
    files {
        foo => "module @; pub struct Bar(int, int)",
        main => "let a = foo::Bar(6, 7); let TEST_VALUE = a.1;",
    } => Int(7),
);

test_vm!(
    nested_module_decl,
    files {
        util => "module game::util; pub const SCALE: int = 3;",
        main => "let TEST_VALUE = game::util::SCALE;",
    } => Int(3),
);

// the parent module's own file must not wipe its children when its rib syncs
test_vm!(
    nested_module_with_declared_parent,
    files {
        game => "module game; pub const BASE: int = 10;",
        util => "module game::util; pub const SCALE: int = 3;",
        main => "let TEST_VALUE = game::BASE + game::util::SCALE;",
    } => Int(13),
);

test_vm!(
    nested_module_use,
    files {
        util => "module game::util; pub fn double(n: int) -> int { n * 2 }",
        main => "use game::util; let TEST_VALUE = util::double(21);",
    } => Int(42),
);
