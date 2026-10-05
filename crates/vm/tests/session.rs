use std::{sync::atomic::Ordering, thread, time::Duration};

use solve::Modules;
use vm::{Session, Vm};

fn session(modules: &[(&str, &str)]) -> Session {
    let mut vm = Vm::new();
    let library = vm.install_library(|_| {});
    let modules = Modules::from_files(modules.iter().copied(), &library);
    Session::new(vm, library, modules)
}

fn echo(session: &mut Session, input: &str) -> String {
    session.run(input).unwrap().expect("an echo")
}

fn quiet(session: &mut Session, input: &str) {
    assert_eq!(session.run(input).unwrap(), None, "{input}");
}

#[test]
fn let_then_echo() {
    let mut session = session(&[]);
    quiet(&mut session, "let a = [1, 2];");
    assert_eq!(echo(&mut session, "a"), "[1, 2]: [int]");
}

#[test]
fn expression_without_semicolon() {
    let mut session = session(&[]);
    assert_eq!(echo(&mut session, "1 + 2"), "3: int");
}

#[test]
fn struct_literal_without_semicolon() {
    let mut session = session(&[]);
    quiet(&mut session, "struct P { x: int }");
    assert_eq!(echo(&mut session, "P { x = 1 }.x"), "1: int");
}

#[test]
fn string_echo_is_quoted() {
    let mut session = session(&[]);
    assert_eq!(echo(&mut session, r#""x""#), "\"x\": str");
}

#[test]
fn unit_echoes_nothing() {
    let mut session = session(&[]);
    quiet(&mut session, "while false {}");
}

#[test]
fn fn_carries() {
    let mut session = session(&[]);
    quiet(&mut session, "fn f(x: int) -> int { x + 1 }");
    assert_eq!(echo(&mut session, "f(1)"), "2: int");
}

#[test]
fn use_carries() {
    let mut session = session(&[(
        "m.mim",
        "module @;
         pub fn one() -> int { 1 }",
    )]);
    quiet(&mut session, "use m;");
    assert_eq!(echo(&mut session, "m::one()"), "1: int");
}

#[test]
fn open_type_pins_down() {
    let mut session = session(&[]);
    quiet(&mut session, "let a = [];");
    assert_eq!(echo(&mut session, "a"), "[]: [_]");
    quiet(&mut session, "a = [1];");
    assert_eq!(echo(&mut session, "a"), "[1]: [int]");
}

#[test]
fn shadowing() {
    let mut session = session(&[]);
    quiet(&mut session, "let a = 1;");
    quiet(&mut session, r#"let a = "x";"#);
    assert_eq!(echo(&mut session, "a"), "\"x\": str");
}

#[test]
fn assignment_reaches_earlier_let() {
    let mut session = session(&[]);
    quiet(&mut session, "let n = 1;");
    quiet(&mut session, "n = n + 1;");
    assert_eq!(echo(&mut session, "n"), "2: int");
}

#[test]
fn redefined_fn_wins_but_old_callers_keep_the_old() {
    let mut session = session(&[]);
    quiet(&mut session, "fn f() -> int { 1 }");
    quiet(&mut session, "fn g() -> int { f() }");
    quiet(&mut session, "fn f() -> int { 2 }");
    assert_eq!(echo(&mut session, "f()"), "2: int");
    assert_eq!(echo(&mut session, "g()"), "1: int");
}

#[test]
fn redefined_struct_keeps_old_values() {
    let mut session = session(&[]);
    quiet(&mut session, "struct P { x: int }");
    quiet(&mut session, "let p = P { x = 1 };");
    quiet(&mut session, "struct P { y: int }");
    assert_eq!(echo(&mut session, "p.x"), "1: int");
    assert_eq!(echo(&mut session, "P { y = 2 }.y"), "2: int");
}

#[test]
fn solve_error_leaves_session() {
    let mut session = session(&[]);
    assert!(session.run("let x: str = 1;").is_err());
    quiet(&mut session, "let x = 2;");
    assert_eq!(echo(&mut session, "x"), "2: int");
}

#[test]
fn module_input_rejected() {
    let mut session = session(&[]);
    assert!(session.run("module m;").is_err());
}

#[test]
fn fault_keeps_runtime_effects() {
    let mut session = session(&[]);
    quiet(&mut session, "let a = [1];");
    let fault = session.run(
        "a[0] = 5;
         let none: int? = null;
         none!",
    );
    assert!(fault.is_err());
    assert_eq!(echo(&mut session, "a"), "[5]: [int]");
}

#[test]
fn fault_drops_its_bindings() {
    let mut session = session(&[]);
    let fault = session.run(
        "let none: int? = null;
         none!",
    );
    assert!(fault.is_err());
    assert!(session.run("none").is_err());
}

#[test]
fn fault_drops_its_items_and_frees_their_ids() {
    let mut session = session(&[]);
    let fault = session.run(
        "fn h() -> int { 1 }
         let none: int? = null;
         none!",
    );
    assert!(fault.is_err());
    quiet(&mut session, "fn h() -> int { 2 }");
    assert_eq!(echo(&mut session, "h()"), "2: int");
}

#[test]
fn interrupt() {
    let mut session = session(&[]);
    let interrupt = session.interrupt();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(10));
        interrupt.store(true, Ordering::Relaxed);
    });
    let interrupted = session.run("loop {}").unwrap_err();
    assert!(
        interrupted.to_string().contains("interrupted"),
        "{interrupted}"
    );
    assert_eq!(echo(&mut session, "1"), "1: int");
}

#[test]
fn interrupt_between_inputs_is_dropped() {
    let mut session = session(&[]);
    session.interrupt().store(true, Ordering::Relaxed);
    quiet(&mut session, "for i in 100000 {}");
}

#[test]
fn later_impl_faults_at_an_earlier_pact_call() {
    let mut session = session(&[]);
    quiet(
        &mut session,
        "pact Area {
             fn area(self) -> int;
         }
         struct Sq { s: int }
         impl Area for Sq {
             fn area(self) -> int {
                 self.s * self.s
             }
         }
         fn measure(a: Area) -> int { a.area() }",
    );
    assert_eq!(echo(&mut session, "measure(Sq { s = 3 })"), "9: int");
    quiet(
        &mut session,
        "struct Circle { r: int }
         impl Area for Circle {
             fn area(self) -> int {
                 3 * self.r * self.r
             }
         }",
    );
    let fault = session.run("measure(Circle { r = 2 })").unwrap_err();
    assert!(
        fault.to_string().contains("implemented the pact"),
        "{fault}"
    );
    assert_eq!(echo(&mut session, "measure(Sq { s = 2 })"), "4: int");
}

#[test]
fn host_can_call_declared_fns() {
    let mut session = session(&[]);
    quiet(&mut session, "fn seven() -> int { 7 }");
    assert_eq!(session.vm().call::<i64>("seven", ()).unwrap(), 7);
}

#[test]
fn unfinished_inputs() {
    assert!(Session::unfinished("fn f() {"));
    assert!(Session::unfinished("let a = [1,"));
    assert!(Session::unfinished("if true {"));
    assert!(Session::unfinished(
        "fn f() {
             1"
    ));
    assert!(!Session::unfinished("let a = 1"));
    assert!(!Session::unfinished("let a = 1;"));
    assert!(!Session::unfinished("let = ;"));
    assert!(!Session::unfinished(
        "fn f() {
             1
         }"
    ));
}
