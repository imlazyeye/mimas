#[macro_use]
mod test_runner;

use std::{cell::RefCell, rc::Rc};

use vm::Vm;

test_run!(
    print_does_not_error,
    r#"{ print(1); print("hi"); 0 }"# => "0",
    r#"{ print([1, 2, 3]); 0 }"# => "0",
);

// panic has type `!`, so a panicking match/if arm coerces to the other arm's type
test_run!(
    panic_arm_coerces_via_never,
    r#"let x: int = if true { 5 } else { panic("nope") };"#,
    "x" => "5",
);

test_fail!(
    library_names_are_not_values,
    "std;",
    "array;",
    "dict;",
    "let a = std::fs;",
    r#"f"{std}";"#,
);

#[test]
fn print_goes_to_the_sink() {
    let lines = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::compile(
        r#"print("hi");
           dbg(1);
           print([1, 2]);"#,
        library::std,
    )
    .unwrap();
    let sink = Rc::clone(&lines);
    vm.fixture::<library::Output>()
        .set(move |line| sink.borrow_mut().push(line.to_string()));
    vm.run().unwrap();
    assert_eq!(*lines.borrow(), ["hi", "dbg value: 1", "[1, 2]"]);
}
