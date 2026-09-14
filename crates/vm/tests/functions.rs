#[macro_use]
mod vm_test_utils;

use vm::{Captured::*, Vm};
use vm_test_utils::{Arities, arity, dynamic_call, kept, with_keep};

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
fn host_passes_typed_args() {
    const SOURCE: &str = "fn add(a: int, b: int) -> int { a + b }";
    let mut vm = vm::Vm::execute(SOURCE, |_| {}).unwrap();
    assert_eq!(vm.call::<i64>("add", (1, 2)).unwrap(), 3);
    let missing = vm.call::<i64>("add", (1,)).unwrap_err();
    assert!(
        missing.to_string().contains("takes 2 arguments, got 1"),
        "{missing}"
    );
    let extra = vm.call::<i64>("add", (1, 2, 3)).unwrap_err();
    assert!(
        extra.to_string().contains("takes 2 arguments, got 3"),
        "{extra}"
    );
    let wrong = vm.call::<i64>("add", (1, "two")).unwrap_err();
    assert!(wrong.to_string().contains("argument 2"), "{wrong}");
    let returns = vm.call::<String>("add", (1, 2)).unwrap_err();
    assert!(returns.to_string().contains("returns `int`"), "{returns}");
    let unknown = vm.call::<i64>("missing", ()).unwrap_err();
    assert!(unknown.to_string().contains("no fn `missing`"), "{unknown}");
}

#[test]
fn host_fills_defaults_and_calls_resolved_fns() {
    const SOURCE: &str = "
        fn greet(name: str, punct = \"!\") -> str { name + punct }
        fn last(xs = [1, 2]) -> int { xs[1] }
    ";
    let mut vm = vm::Vm::execute(SOURCE, |_| {}).unwrap();
    assert_eq!(vm.call::<String>("greet", ("hi", "?")).unwrap(), "hi?");
    assert_eq!(vm.call::<i64>("last", ()).unwrap(), 2);
    let none = vm.call::<String>("greet", ()).unwrap_err();
    assert!(
        none.to_string().contains("takes 1 to 2 arguments, got 0"),
        "{none}"
    );
    let greet = vm.root().function("greet").unwrap().clone();
    greet
        .check("greet", &[Some(vm::Ty::Str)], Some(&vm::Ty::Str))
        .unwrap();
    for _ in 0..3 {
        let (s, ()) = vm
            .call_function::<String, _>(&greet, ("hi",), |_| ())
            .unwrap();
        assert_eq!(s, "hi!");
    }
}

#[test]
fn host_calls_return_units_and_options() {
    const SOURCE: &str = "
        fn nothing() {}
        fn maybe(flag: bool) -> int? { if flag 4 else null }
    ";
    let mut vm = vm::Vm::execute(SOURCE, |_| {}).unwrap();
    vm.call::<()>("nothing", ()).unwrap();
    assert_eq!(vm.call::<Option<i64>>("maybe", (true,)).unwrap(), Some(4));
    assert_eq!(vm.call::<Option<i64>>("maybe", (false,)).unwrap(), None);
}

#[test]
fn host_sees_root_items_but_not_methods() {
    const SOURCE: &str = "
        const LIMIT = 3;
        struct Foo;
        impl Foo {
            fn value(self) -> int { 2 }
            fn only_method(self) -> int { 3 }
        }
        fn value() -> int { 1 }
        fn inner() -> int { const LIMIT = 99; const INNER = 7; LIMIT + INNER }
    ";
    let mut vm = vm::Vm::execute(SOURCE, |api| {
        api.module("lib")
            .constant("K", vm::Ty::Int, vm::Literal::Int(1), "");
        api.constant("ROOT_K", vm::Ty::Int, vm::Literal::Int(2), "");
    })
    .unwrap();
    assert_eq!(vm.call::<i64>("value", ()).unwrap(), 1);
    assert_eq!(vm.call::<i64>("inner", ()).unwrap(), 106);
    assert!(vm.root().function("only_method").is_none());
    assert_eq!(vm.root().constants.len(), 1);
    assert_eq!(vm.root().constants["LIMIT"], vm::Constant::Int(3));
    assert!(vm.root().modules.is_empty());
    let names: Vec<&str> = vm.root().functions.keys().map(String::as_str).collect();
    assert_eq!(names, ["value", "inner"]);
}

#[test]
fn host_reaches_module_items_by_path() {
    let files = &[
        (
            "util",
            "module @; pub const STEP = 2; pub fn value() -> int { 2 } fn hidden() -> int { 3 }",
        ),
        ("deep", "module util::deep; pub fn value() -> int { 4 }"),
        ("main", "use util; fn value() -> int { 1 }"),
    ];
    let mut vm = vm::Vm::execute_files(files, |_| {}).unwrap();
    assert_eq!(vm.call::<i64>("value", ()).unwrap(), 1);
    assert_eq!(vm.call::<i64>("util::value", ()).unwrap(), 2);
    assert_eq!(vm.call::<i64>("util::deep::value", ()).unwrap(), 4);
    assert_eq!(vm.call::<i64>("util::hidden", ()).unwrap(), 3);
    let util = vm.root().module("util").unwrap();
    assert_eq!(util.function("hidden").unwrap().vis, vm::Vis::Private);
    assert_eq!(util.function("value").unwrap().vis, vm::Vis::Public);
    assert!(vm.root().function("hidden").is_none());
    assert_eq!(util.constants["STEP"], vm::Constant::Int(2));
    assert!(util.module("deep").is_some());
    assert!(vm.root().module("std").is_none());
}

#[test]
fn host_reaches_declared_items() {
    const SOURCE: &str = "
        struct Tally { n: int }
        impl Tally {
            fn bump(self, by: int) -> int { self.n += by; self.n }
            fn make() -> Tally { Tally { n = 0 } }
        }
        enum Mood { Good, Bad }
        impl Mood { fn label(self) -> str { \"mood\" } }
        struct Pair(int);
    ";
    let vm = vm::Vm::execute(SOURCE, |_| {}).unwrap();
    let tally = vm.root().ty("Tally").unwrap();
    assert_eq!(tally.fields, ["n"]);
    let names: Vec<&str> = tally.methods.keys().map(String::as_str).collect();
    assert_eq!(names, ["bump", "make"]);
    // a method counts its own `self` as parameter 0, an assoc fn has none
    assert_eq!(
        vm.root()
            .method("Tally::bump")
            .unwrap()
            .header
            .parameters
            .len(),
        2
    );
    assert_eq!(
        vm.root()
            .method("Tally::make")
            .unwrap()
            .header
            .parameters
            .len(),
        0
    );
    // a tuple struct names its members by index, an enum has no fields of its own
    assert_eq!(vm.root().ty("Pair").unwrap().fields, ["0"]);
    assert!(vm.root().ty("Mood").unwrap().fields.is_empty());
    assert!(vm.root().method("Mood::label").is_some());
    // a method isn't a fn of the module that declared it
    assert!(vm.root().function("bump").is_none());
}

#[test]
fn host_call_keeps_entry_registers() {
    const SOURCE: &str = "
        let a = 7;
        let b = 8;
        fn f() -> int { 99 }
    ";
    let mut vm = vm::Vm::execute(SOURCE, |_| {}).unwrap();
    assert_eq!(vm.call::<i64>("f", ()).unwrap(), 99);
    assert_eq!(vm.resolve_name("a"), Some(Int(7)));
    assert_eq!(vm.resolve_name("b"), Some(Int(8)));
}

#[test]
fn host_call_then_runs_in_the_arena_and_faults_recover() {
    const SOURCE: &str = "
        fn seven() -> int { 7 }
        fn boom() -> int { let xs = [1]; xs[5] }
    ";
    let mut vm = vm::Vm::execute(SOURCE, |_| {}).unwrap();
    let (n, built) = vm
        .call_then::<i64, _>("seven", (), |ctx| {
            vm::Val::Array(ctx.new_array(vec![vm::Val::Int(1)])).capture()
        })
        .unwrap();
    assert_eq!(n, 7);
    assert_eq!(built, Array(vec![Int(1)]));
    for _ in 0..3 {
        let faulted =
            vm.call_then::<i64, _>("boom", (), |_| unreachable!("then ran after a fault"));
        assert!(faulted.is_err());
    }
    assert_eq!(vm.call::<i64>("seven", ()).unwrap(), 7);
}

#[test]
fn dynamic_calls_fault() {
    const F: &str = "let f = untyped(|n: int| { n + 1; });";
    let many = dynamic_call(&format!("{F} f(1, 2, 3);")).unwrap_err();
    assert!(many.contains("takes 1 arguments, got 3"), "{many}");
    let few = dynamic_call(&format!("{F} f();")).unwrap_err();
    assert!(few.contains("takes 1 arguments, got 0"), "{few}");
    dynamic_call(&format!("{F} f(1);")).unwrap();

    for source in ["let f = untyped(4); f();", "let f = untyped(null); f();"] {
        let err = dynamic_call(source).unwrap_err();
        assert!(err.contains("isn't a function"), "{source}: {err}");
    }
}

#[test]
fn host_call_stashed_closure_cross_collection() {
    const SOURCE: &str = "
        let step = 10;
        keep(|n: int| -> int { n + step });
        fn churn() -> int { 
            let n = 0; 
            for i in 500 { 
                let xs = [i, i, i]; 
                n += xs[2]; 
            } 
            n 
        }
    ";
    let mut vm = with_keep(SOURCE);
    let add_step = kept(&vm);
    assert_eq!(vm.call_value::<i64>(&add_step, (5,)).unwrap(), 15);
    // we're basically just forcing a gc run
    for _ in 0..8 {
        vm.call::<i64>("churn", ()).unwrap();
    }
    assert_eq!(vm.call_value::<i64>(&add_step, (1,)).unwrap(), 11);
}

#[test]
fn host_calls_stashed_method() {
    const SOURCE: &str = "
        struct Tally { n: int }
        impl Tally { 
            fn bump(self, by: int) -> int { 
                self.n += by; 
                self.n 
            } 
        }
        let tally = Tally { n = 1 };
        keep(Tally::bump);
        keep(tally);
    ";
    let mut vm = with_keep(SOURCE);
    let (bump, tally) = (kept(&vm), kept(&vm));
    assert_eq!(vm.call_value::<i64>(&bump, (&tally, 2)).unwrap(), 3);
}

#[test]
fn value_reports_its_own_sig() {
    const SOURCE: &str = "
        struct Tally { n: int }
        impl Tally { 
            fn bump(self, by: int) -> int { 
                self.n += by; 
                self.n 
            } 
        }
        struct Pair(int);
        fn two(a: int, b: int) -> int { a + b }
        arity(two);
        arity(Tally::bump);
        arity(|n: int| { n });
        arity(Pair);
        arity(4);
    ";
    let vm = Vm::execute(SOURCE, |api| api.add_named("arity", arity)).unwrap();
    let noted = vm.fixture::<Arities>();
    assert_eq!(
        *noted.0.borrow(),
        [Some(2), Some(2), Some(1), None, None],
        "fn, method, closure, ctor, int"
    );
}
#[test]
fn ctor_value_has_no_body() {
    const F: &str = "struct Pair(int); let f = untyped(Pair);";
    for call in ["f();", "f(1);"] {
        let err = dynamic_call(&format!("{F} {call}")).unwrap_err();
        assert!(err.contains("isn't a function"), "{call}: {err}");
    }
}

#[test]
fn value_with_no_body_is_rejected() {
    const SOURCE: &str = "
        struct Pair(int);
        keep(Pair);
        fn seven() -> int { 7 }
    ";
    let mut vm = with_keep(SOURCE);
    let ctor = kept(&vm);
    let err = vm.call_value::<i64>(&ctor, (1,)).unwrap_err();
    assert!(err.to_string().contains("no function body"), "{err}");
    assert_eq!(vm.call::<i64>("seven", ()).unwrap(), 7);
}

#[test]
fn fault_through_call_value() {
    const SOURCE: &str = "
        keep(|| { let xs = [1]; xs[5] });
        fn seven() -> int { 7 }
    ";
    let mut vm = with_keep(SOURCE);
    let boom = kept(&vm);
    for _ in 0..3 {
        let faulted =
            vm.call_value_then::<i64, _>(&boom, (), |_| unreachable!("then ran after a fault"));
        assert!(faulted.is_err());
    }
    assert_eq!(vm.call::<i64>("seven", ()).unwrap(), 7);
}

// regression check for #10
test_vm!(
    assoc_fn_called_on_self,
    "struct Board { n: int }
     struct Puzzle {}
     impl Puzzle {
         fn count(board: Board) -> int { board.n }
         fn run(self, board: Board) -> int { self.count(board) }
     }
     let p = Puzzle {};",
    "p.run(Board { n = 7 })" => Int(7),
);

test_vm!(
    assoc_fn_called_on_value,
    "struct S { x: int }
     impl S { fn make(a: int, b = 10) -> int { a + b } }
     enum E { A, B }
     impl E { fn pick(n: int) -> int { n * 2 } }
     let s = S { x = 1 };
     let maybe: S? = s;
     let e = E::B;",
    "s.make(1)" => Int(11),
    "s.make(1, b = 2)" => Int(3),
    "maybe?.make(4)" => Int(14),
    "e.pick(4)" => Int(8),
);
