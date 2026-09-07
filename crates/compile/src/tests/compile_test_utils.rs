#[cfg(test)]
pub(crate) fn lower_ir(src: &str) -> String {
    let lexer = parse::lex::Lexer::new(src, 0, String::new());
    let ast = parse::Parser::new(lexer).into_ast().unwrap();
    let mut solver = solve::Solver::new();
    solver.solve(&ast).unwrap();
    let resolutions = solver.into();
    let mut ir = crate::Ir::new(resolutions, Default::default());
    ir.lower(&ast.unpack());
    ir.to_string()
}

/// The fully-compiled op stream (post-optimization) for `src`, for asserting codegen/optimizer
/// behavior that a value-only VM test can't observe (same result, different ops).
#[cfg(test)]
pub(crate) fn compile_ops(src: &str) -> Vec<crate::Op> {
    let lexer = parse::lex::Lexer::new(src, 0, String::new());
    let ast = parse::Parser::new(lexer).into_ast().unwrap();
    let mut solver = solve::Solver::new();
    solver.solve(&ast).unwrap();
    let resolutions = solver.into();
    let mut ir = crate::Ir::new(resolutions, Default::default());
    ir.lower(&ast.unpack());
    let mut compiler = crate::Compiler::new();
    let _ = compiler.compile(ir);
    compiler.ops
}

#[macro_export]
macro_rules! test_compile {
    ($(#[$attr:meta])* $name:ident, files { $($file_name:ident => $source:expr),+ $(,)? }, $expected:expr) => {
        #[cfg(test)]
        #[test]
        $(#[$attr])*
        fn $name() {
            let mut solver = solve::Solver::new();
            let mut stmts = Vec::new();

            $(
                let lexer = parse::lex::Lexer::new($source, 0, stringify!($file_name).into());
                let ast = parse::Parser::new(lexer).into_ast().unwrap();
                solver.solve(&ast).unwrap();
                stmts.extend(ast.unpack());
            )+

            let resolutions = solver.into();

            let mut ir = $crate::Ir::new(resolutions, Default::default());
            ir.lower(&stmts);

            pretty_assertions::assert_eq!(ir.to_string(), $expected);
        }
    };
    ($(#[$attr:meta])* $name:ident, $source:expr, $expected:expr) => {
        #[cfg(test)]
        #[test]
        $(#[$attr])*
        fn $name() {

            let lexer = parse::lex::Lexer::new($source, 0, String::new());
            let ast = parse::Parser::new(lexer).into_ast().unwrap();
            let mut solver = solve::Solver::new();
            solver.solve(&ast).unwrap();
            let resolutions = solver.into();

            let mut ir = $crate::Ir::new(resolutions, Default::default());
            ir.lower(&ast.unpack());

            pretty_assertions::assert_eq!(ir.to_string(), $expected);
        }
    }
}

/// asserts the compiled (post-optimization) op stream for `$src` contains at least one op matching
/// `$pat` (use `|` to accept several shapes). for counts, absence, or nested-field checks write a
/// plain `#[test]` with `compile_ops`.
#[macro_export]
macro_rules! test_lowering {
    ($(#[$attr:meta])* $name:ident, $src:expr, $($pat:pat_param)|+ $(,)?) => {
        #[cfg(test)]
        #[test]
        $(#[$attr])*
        fn $name() {
            let ops = $crate::tests::compile_test_utils::compile_ops($src);
            assert!(
                ops.iter().any(|o| matches!(o, $($pat)|+)),
                "expected an op matching `{}` in:\n{:#?}",
                stringify!($($pat)|+),
                ops,
            );
        }
    };
}
