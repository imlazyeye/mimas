use parse::{Parser, lex::Lexer};
use solve::{Resolutions, Solver};

use crate::{BodyId, Compiler, Ir, Program, Reg};

fn session() -> (Solver, Ir, Compiler) {
    let solver = Solver::new();
    let ir = Ir::new(Resolutions::from(solver.clone()), Default::default());
    (solver, ir, Compiler::new())
}

// one input: solve it on top of the earlier ones, lower it into a fresh entry (after the first)
// and compile what's new
fn input(
    (solver, ir, compiler): &mut (Solver, Ir, Compiler),
    src: &str,
    from: Option<BodyId>,
) -> Program {
    let ast = Parser::new(Lexer::new(src, 0, String::new()))
        .try_into_ast()
        .unwrap();
    solver.solve(&ast).unwrap();
    let resolutions = Resolutions::from(solver.clone());
    match from {
        Some(from) => {
            ir.new_entry(from, resolutions);
        }
        None => ir.resolutions = resolutions,
    }
    ir.lower_echo(ast.stmts());
    compiler.compile(ir)
}

#[test]
fn second_input_extends_bytes() {
    let mut session = session();
    let p1 = input(&mut session, "let a = 1;", None);
    let p2 = input(&mut session, "let b = a + 1;", Some(p1.entry));
    assert!(p2.bytes.starts_with(&p1.bytes));
    assert!(p2.chunks.len() > p1.chunks.len());
    assert_ne!(p2.entry, p1.entry);
    assert_eq!(p2.chunks[p2.entry].offset, p1.bytes.len());
    for (body, chunk) in p1.chunks.iter() {
        assert_eq!(p2.chunks[body].offset, chunk.offset);
    }
}

#[test]
fn signatures_cover_every_body() {
    let mut session = session();
    let p1 = input(&mut session, "fn f() -> int { 1 }", None);
    let p2 = input(
        &mut session,
        "fn f() -> int { 2 }
         let x = f();",
        Some(p1.entry),
    );
    assert_eq!(p2.signatures.len(), p2.chunks.len());
    assert!(p2.signatures[p1.root.functions["f"].body].is_some());
}

#[test]
fn shadowed_name_resolves_to_the_latest() {
    let program = input(
        &mut session(),
        r#"let a = 1;
           let a = "x";"#,
        None,
    );
    assert_eq!(program.chunks[program.entry].locals["a"], Reg::from(1));
}
