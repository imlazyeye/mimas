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
