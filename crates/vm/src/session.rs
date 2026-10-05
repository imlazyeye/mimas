use std::sync::{Arc, atomic::AtomicBool};

use ::api::Library;
use compile::{BodyId, Compiler, Ir};
use parse::{
    Ast, Parser, Stmt, StmtKind,
    errors::{MissingSemiColon, UnexpectedEnd},
    lex::{Lexer, is_module},
};
use shared::{Error, Ty};
use solve::{Modules, Resolutions, traits::Query};

use crate::Vm;

/// A program built one input at a time. Each input is solved, compiled and run on top of the ones
/// before it with their bindings, items and runtime state in place.
pub struct Session {
    vm: Vm,
    modules: Modules,
    ir: Ir,
    compiler: Compiler,
    entry: BodyId,
    interrupt: Arc<AtomicBool>,
}

impl Session {
    /// Starts a session over `modules`, which must have solved cleanly against `library`, the
    /// library installed into `vm`.
    pub fn new(mut vm: Vm, library: Library<()>, modules: Modules) -> Self {
        assert!(
            modules.errors.is_empty(),
            "a session's modules must solve cleanly"
        );
        let (ir, compiler) = vm.load_modules(library, &modules);
        Self {
            vm,
            modules,
            ir,
            compiler,
            entry: BodyId::ZERO,
            interrupt: Arc::default(),
        }
    }

    /// Whether `input` stops before its code does (an open brace, an unfinished statement). A repl
    /// reads another line before running such an input.
    pub fn unfinished(input: &str) -> bool {
        // a block that ends in a bare expression reports its missing `;` and drops the missing `}`
        // (one error per token), which the `;` that `run` appends anyway brings back
        [input.to_string(), format!("{input};")].iter().any(|text| {
            let (_, errors) = parse(text);
            !errors.is_empty()
                && errors
                    .iter()
                    .all(|error| error.downcast_ref::<UnexpectedEnd>().is_some())
        })
    }

    /// Solves, compiles and runs `input` on top of the inputs before it. A trailing expression
    /// comes back shown with its type (`[1, 2]: [int]`), unless it's unit.
    pub fn run(&mut self, input: &str) -> Result<Option<String>, Error> {
        if is_module(input) {
            return Err(Error::msg("an input can't be a module"));
        }
        // most inputs are a bare expression, which reads better without the `;` a statement needs
        let (_, errors) = parse(input);
        let text = if !errors.is_empty()
            && errors
                .iter()
                .all(|error| error.downcast_ref::<MissingSemiColon>().is_some())
        {
            format!("{input};")
        } else {
            input.to_string()
        };
        let mut next = self.modules.load([("<repl>", text.as_str())]);
        if !next.errors.is_empty() {
            return Err(next.errors.swap_remove(0));
        }
        let resolutions = Resolutions::from(next.solver.clone());
        let stmts = next.asts[0].stmts();
        let echo_ty = match stmts.last().map(Stmt::kind) {
            Some(StmtKind::Expr(expr)) => Some(expr.query(&mut next.solver)?)
                .filter(|ty| *ty != Ty::Unit)
                .map(|ty| ty.display(&resolutions)),
            _ => None,
        };
        // a faulted input's dec ids come back around in the next one, which must not find these
        // bodies
        let items = self.ir.item_bodies.clone();
        let entry = self.ir.new_entry(self.entry, resolutions);
        self.ir.lower_echo(stmts);
        self.vm.extend(self.compiler.compile(&mut self.ir));
        match self
            .vm
            .run_then(&self.interrupt, |ctx, value| ctx.display(value))
        {
            Ok(value) => {
                self.modules = next;
                self.entry = entry;
                Ok(echo_ty.map(|ty| format!("{value}: {ty}")))
            }
            Err(error) => {
                self.ir.item_bodies = items;
                Err(error)
            }
        }
    }

    /// A flag that interrupts the input that's running when set, from a signal handler or another
    /// thread. It's cleared once the interrupt lands.
    pub fn interrupt(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.interrupt)
    }

    /// The vm the inputs run in, for its fixtures and for calling the fns they declared. After a
    /// faulted input, `resolve_name` still sees that input's bindings until the next one runs.
    pub fn vm(&mut self) -> &mut Vm {
        &mut self.vm
    }
}

fn parse(input: &str) -> (Ast, Vec<Error>) {
    Parser::new(Lexer::new(input, 0, "<repl>".into())).into_ast()
}
