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
    set_instance_field,
    "struct Foo { x: int }
    let foo = Foo { x = 42 };
    foo.x = 0;",
    "foo.x" => Int(0)
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

// `print`/`display`/f-strings render an instance as `Name { .. }`, not the bare `@id { .. }`
// the runtime used to fall back to before struct/variant names were threaded through to the VM.
test_vm!(
    instance_display_uses_struct_name,
    "struct Node { next: Node?, prev: Node?, val: int }
     let n = Node { next = null, prev = null, val = 3 };",
    r#"f"{n}""# => str!("Node { null, null, 3 }"),
);

test_vm!(
    tuple_struct_display_uses_struct_name,
    "struct Wrap(int);
     let w = Wrap(9);",
    r#"f"{w}""# => str!("Wrap { 9 }"),
);

// enum variant instances display qualified as `Enum::Variant { .. }`
test_vm!(
    enum_variant_display_uses_qualified_name,
    "enum Shape { Circle { radius: int }, Square(int) }",
    r#"f"{Shape::Circle { radius = 5 }}""# => str!("Shape::Circle { 5 }"),
    r#"f"{Shape::Square(4)}""# => str!("Shape::Square { 4 }"),
);
