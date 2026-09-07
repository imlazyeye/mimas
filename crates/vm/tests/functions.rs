#[macro_use]
mod vm_test_utils;

use vm::Captured::*;

test_vm!(
    return_int,
    "fn foo() -> int { 0 }",
    "foo()" => Int(0),
);

test_vm!(
    optional_param_default_used,
    "fn foo(a: int, b: int = 10) -> int { a + b }",
    "foo(5)" => Int(15),
);

test_vm!(
    fn_default_with_expression,
    "fn add(a: int = 1 + 1) -> int { a }",
    "add()" => Int(2),
    "add(5)" => Int(5),
);

test_vm!(
    function_param_inferred_from_return,
    "fn id(a) -> int { a }",
    "id(7)" => Int(7),
);

test_vm!(
    fn_recursive,
    "fn fact(n: int) -> int { if n <= 1 { 1 } else { n * fact(n - 1) } }",
    "fact(5)" => Int(120),
);

test_vm!(
    fn_early_return,
    "fn foo(a: int) -> int {
        if a > 0 {
            return a * 2;
        }
        a
    }",
    "foo(5)" => Int(10),
    "foo(-1)" => Int(-1),
);

test_vm!(
    fn_return_from_nested_block,
    "fn foo() -> int {
        {
            return 42;
        }
        0
    }",
    "foo()" => Int(42),
);

test_vm!(
    fn_return_from_loop,
    "fn foo() -> int {
        loop {
            return 42;
        }
    }",
    "foo()" => Int(42),
);

test_vm!(
    fn_mutual,
    "fn even(n: int) -> bool { if n == 0 { true } else { odd(n - 1) } }
     fn odd(n: int) -> bool { if n == 0 { false } else { even(n - 1) } }",
    "even(10)" => Bool(true),
    "odd(7)" => Bool(true),
);

test_vm!(
    fn_higher_order,
    "fn higher() {}
     fn order() { higher() }",
    "order()" => Null,
);

test_vm!(
    closure,
    "let f = |x: int| -> int { x + 1 };",
    "f(5)" => Int(6),
);

test_vm!(
    closure_capture_int,
    "let x = 10;
     let add_x = |y: int| -> int { x + y };",
    "add_x(5)" => Int(15),
);

test_vm!(
    closure_multi_capture,
    "let a = 1;
     let b = 2;
     let c = 3;
     let add_abc = |d: int| -> int { a + b + c + d };",
    "add_abc(4)" => Int(10),
);

test_vm!(
    closure_returned_from_fn,
    "fn make_adder(n: int) -> (int) -> int { |x: int| -> int { x + n } }
     let add5 = make_adder(5);",
    "add5(3)" => Int(8),
    "add5(10)" => Int(15),
);

#[test]
fn host_function_call() {
    const SOURCE: &str = "fn foo() -> int { 1 }";
    let mut vm = vm::Vm::execute(SOURCE, |_| {}).unwrap();
    assert_eq!(vm.call_fn("foo"), Some(vm::Captured::Int(1)));
}
