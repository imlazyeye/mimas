use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use vm::Vm;

#[test]
fn run_then_yields_null_for_a_script() {
    let mut vm = Vm::compile("let a = 1;", |_| {}).unwrap();
    let shown = vm
        .run_then(&AtomicBool::new(false), |ctx, value| ctx.display(value))
        .unwrap();
    assert_eq!(shown, "null");
}

#[test]
fn const_cache_reloads() {
    let mut vm = Vm::new();
    let library = vm.install_library(|_| {});
    for value in [2, 5] {
        let source = format!(
            "const TABLE = [0, {value}];
             fn second() -> int {{
                 TABLE[1]
             }}"
        );
        let modules = solve::Modules::from_files([("test", source.as_str())], &library);
        assert!(modules.errors.is_empty(), "{:?}", modules.errors);
        let mut ir = compile::Ir::new(
            solve::Resolutions::from(modules.solver.clone()),
            Default::default(),
        );
        ir.lower(modules.asts.iter().flat_map(parse::Ast::stmts));
        let program = compile::Compiler::new().compile(&mut ir);
        vm.set_fuel(Some(0));
        vm.load_program(program);
        assert_eq!(vm.fuel(), Some(0));
        vm.set_fuel(None);
        assert_eq!(vm.call::<i64>("second", ()).unwrap(), value);
    }
}

#[test]
fn interrupt_faults_and_leaves_the_vm_usable() {
    let mut vm = Vm::compile(
        "fn f() -> int { 7 }
         loop {}",
        |_| {},
    )
    .unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(10));
        flag.store(true, Ordering::Relaxed);
    });
    let interrupted = vm.run_then(&stop, |_, _| ()).unwrap_err();
    assert!(
        interrupted.to_string().contains("interrupted"),
        "{interrupted}"
    );
    assert_eq!(vm.call::<i64>("f", ()).unwrap(), 7);
}

#[test]
fn fault_leaves_the_vm_usable() {
    let mut vm = Vm::compile(
        "fn boom(d: int) -> int { 1 ~/ d }
         fn ok() -> int { 7 }
         let x = boom(0);",
        |_| {},
    )
    .unwrap();
    assert!(vm.run().is_err());
    assert_eq!(vm.call::<i64>("ok", ()).unwrap(), 7);
    assert!(vm.resolve_name("x").is_some());
}
