use vm::Vm;

const COUNTER: &str = "fn count(n: int) -> int {
         let i = 0;
         while i < n {
             i += 1;
         }
         i
     }
     fn spin() {
         loop {}
     }
     fn seven() -> int {
         7
     }";

fn counter() -> Vm {
    let mut vm = Vm::compile(COUNTER, |_| {}).unwrap();
    vm.run().unwrap();
    vm
}

fn spent(vm: &mut Vm, fuel: u64, run: impl FnOnce(&mut Vm)) -> u64 {
    vm.set_fuel(Some(fuel));
    run(vm);
    fuel - vm.fuel().unwrap()
}

#[test]
fn default_no_limit() {
    let mut vm = counter();
    assert_eq!(vm.fuel(), None);
    assert_eq!(vm.call::<i64>("count", (100_000,)).unwrap(), 100_000);
    assert_eq!(vm.fuel(), None);
}

#[test]
fn none_clears_limit() {
    let mut vm = counter();
    vm.set_fuel(Some(10));
    vm.set_fuel(None);
    assert_eq!(vm.call::<i64>("count", (10_000,)).unwrap(), 10_000);
    assert_eq!(vm.fuel(), None);
}

#[test]
fn fueled_call() {
    let mut vm = counter();
    vm.set_fuel(Some(1_000_000));
    assert_eq!(vm.call::<i64>("count", (1_000,)).unwrap(), 1_000);
    let left = vm.fuel().unwrap();
    assert!(left > 0 && left < 1_000_000 - 1_000, "{left}");
}

#[test]
fn distributed_spending() {
    let mut vm = counter();
    let once = spent(&mut vm, 1_000_000, |vm| {
        vm.call::<i64>("seven", ()).unwrap();
    });
    vm.call::<i64>("seven", ()).unwrap();
    assert_eq!(1_000_000 - vm.fuel().unwrap(), once * 2);
}

#[test]
fn exact_fuel_works() {
    let mut vm = counter();
    let once = spent(&mut vm, 1_000_000, |vm| {
        vm.call::<i64>("count", (50,)).unwrap();
    });
    vm.set_fuel(Some(once));
    assert_eq!(vm.call::<i64>("count", (50,)).unwrap(), 50);
    assert_eq!(vm.fuel(), Some(0));
    vm.set_fuel(Some(once - 1));
    assert!(vm.call::<i64>("count", (50,)).is_err());
}

#[test]
fn loop_in_a_call() {
    let mut vm = counter();
    vm.set_fuel(Some(5_000));
    let err = vm.call::<()>("spin", ()).unwrap_err();
    assert!(err.to_string().contains("ran out of fuel"), "{err}");
    assert_eq!(vm.fuel(), Some(0));
}

#[test]
fn loop_at_top_level() {
    let mut vm = Vm::compile("loop {}", |_| {}).unwrap();
    vm.set_fuel(Some(5_000));
    let err = vm.run().unwrap_err();
    assert!(err.to_string().contains("ran out of fuel"), "{err}");
    assert_eq!(vm.fuel(), Some(0));
}

#[test]
fn top_level_spans_batches() {
    let source = "let i = 0;
         while i < 100_000 {
             i += 1;
         }";
    let mut vm = Vm::compile(source, |_| {}).unwrap();
    vm.set_fuel(Some(3_000));
    assert!(vm.run().is_err());
    assert_eq!(vm.fuel(), Some(0));
    let mut vm = Vm::compile(source, |_| {}).unwrap();
    vm.set_fuel(Some(10_000_000));
    vm.run().unwrap();
    let left = vm.fuel().unwrap();
    assert!(left > 0 && left < 10_000_000 - 100_000, "{left}");
}

#[test]
fn vm_usable_after_call_runs_out() {
    let mut vm = counter();
    vm.set_fuel(Some(5_000));
    assert!(vm.call::<()>("spin", ()).is_err());
    vm.set_fuel(Some(5_000));
    assert_eq!(vm.call::<i64>("seven", ()).unwrap(), 7);
    assert!(vm.call::<()>("spin", ()).is_err());
    vm.set_fuel(None);
    assert_eq!(vm.call::<i64>("count", (10,)).unwrap(), 10);
}

#[test]
fn vm_usable_after_top_level_runs_out() {
    let mut vm = Vm::compile(
        "fn seven() -> int { 7 }
         loop {}",
        |_| {},
    )
    .unwrap();
    vm.set_fuel(Some(5_000));
    assert!(vm.run().is_err());
    vm.set_fuel(None);
    assert_eq!(vm.call::<i64>("seven", ()).unwrap(), 7);
}

#[test]
fn spent_fuel_faults_every_call() {
    let mut vm = counter();
    vm.set_fuel(Some(0));
    let err = vm.call::<i64>("seven", ()).unwrap_err();
    assert!(err.to_string().contains("ran out of fuel"), "{err}");
    assert!(vm.call::<i64>("seven", ()).is_err());
}
