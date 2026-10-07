//! The book's "try it out" terminal: a [`Session`] behind wasm-bindgen, driven from a web worker.

use std::{cell::RefCell, rc::Rc};

use library::Output;
use miette::{GraphicalReportHandler, GraphicalTheme};
use solve::Modules;
use vm::{Session, Vm};
use wasm_bindgen::prelude::*;

/// A session over the sandboxed std, keeping what each input prints until the input is done.
#[wasm_bindgen]
pub struct Repl {
    session: Session,
    printed: Rc<RefCell<String>>,
}

/// What one input did: what it printed, the echo of a trailing expression, and the error it
/// stopped on.
#[wasm_bindgen(getter_with_clone)]
pub struct Outcome {
    pub printed: String,
    pub echo: Option<String>,
    pub error: Option<String>,
}

#[wasm_bindgen]
impl Repl {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Repl {
        let mut vm = Vm::new();
        let library = vm.install_library(library::sandboxed);
        let printed = Rc::new(RefCell::new(String::new()));
        let sink = Rc::clone(&printed);
        vm.fixture::<Output>().set(move |line| {
            let mut printed = sink.borrow_mut();
            printed.push_str(line);
            printed.push('\n');
        });
        let modules = Modules::from_files(std::iter::empty::<(&str, &str)>(), &library);
        Repl {
            session: Session::new(vm, library, modules),
            printed,
        }
    }

    pub fn unfinished(input: &str) -> bool {
        Session::unfinished(input)
    }

    pub fn run(&mut self, input: &str) -> Outcome {
        let result = self.session.run(input);
        let printed = std::mem::take(&mut *self.printed.borrow_mut());
        match result {
            Ok(echo) => Outcome {
                printed,
                echo,
                error: None,
            },
            Err(report) => {
                let mut error = String::new();
                GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor())
                    .with_width(80)
                    .render_report(&mut error, report.as_ref())
                    .expect("writing to a string");
                Outcome {
                    printed,
                    echo: None,
                    error: Some(error),
                }
            }
        }
    }
}
