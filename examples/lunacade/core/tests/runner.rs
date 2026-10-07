#![allow(dead_code)]

use lunacade_core::{Cart, Diagnostic, Input, Machine};

pub fn cart(main: &str) -> Cart {
    Cart {
        files: [("main.mim".to_owned(), main.to_owned())].into(),
    }
}

pub fn machine(main: &str) -> Machine {
    match Machine::load(&cart(main), 1) {
        Ok(machine) => machine,
        Err(problems) => panic!("the cart didn't load: {problems:?}"),
    }
}

pub fn problems(main: &str) -> Vec<Diagnostic> {
    match Machine::load(&cart(main), 1) {
        Ok(_) => panic!("the cart loaded"),
        Err(problems) => problems,
    }
}

pub fn run(main: &str, frames: usize) -> Vec<String> {
    let mut machine = machine(main);
    for _ in 0..frames {
        machine.frame(&Input::default());
    }
    machine.take_output()
}
