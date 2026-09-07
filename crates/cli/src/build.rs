use api::{Intrinsic, NativeId};
use parse::{Ast, Parser, Stmt, lex::Lexer};
use solve::{Error as SolveError, Solver};
use std::{collections::HashMap, path::Path, sync::Arc};
use walkdir::WalkDir;

pub fn solve(path: &Path) -> (vm::Vm, SolveSummary) {
    let mut sources = vm::Sources::new();
    let mut errors: Vec<SolveError> = vec![];
    let mut io_errors: Vec<std::io::Error> = vec![];
    let mut lines_parsed = 0;
    let mut files: Vec<(std::path::PathBuf, Ast)> = vec![];
    let mut has_main = false;

    let mut vm = vm::Vm::new();
    let library = vm.install_library(library::std);

    let mut solver = Solver::new();
    solver.install_library(&library);

    for entry in WalkDir::new(path) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                io_errors.push(std::io::Error::other(format!(
                    "{}: {}",
                    path.display(),
                    e.into_io_error()
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "walkdir error".into()),
                )));
                continue;
            }
        };
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|e| e.to_str()) != Some("mim")
        {
            continue;
        }

        let file_path = entry.path();
        let mimas = match std::fs::read_to_string(file_path) {
            Ok(mimas) => mimas,
            Err(e) => {
                io_errors.push(std::io::Error::new(
                    e.kind(),
                    format!("{}: {e}", file_path.display()),
                ));
                continue;
            }
        };

        let stem = file_path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if stem == "main" {
            has_main = true;
        }

        lines_parsed += mimas.lines().count();
        let file_id = sources.len();
        sources.insert(
            file_id,
            miette::NamedSource::new(file_path.to_str().unwrap(), Arc::from(mimas.as_str())),
        );
        // full path, not the stem: this is the name parse diagnostics render, and solve/runtime
        // already use the full path -- `module @` stem-ifies it on its own
        let lexer = Lexer::new(&mimas, file_id, file_path.to_str().unwrap().to_string());
        match Parser::new(lexer).into_ast() {
            Ok(ast) => files.push((file_path.to_path_buf(), ast)),
            Err(e) => errors.push(e),
        }
    }

    // directory mode requires main.mim as the entry. single-file mode is whatever the user pointed
    // at. todo, enforce that non-main files in directory mode are pure module declarations.
    if path.is_dir() && !has_main {
        io_errors.push(std::io::Error::other(format!(
            "directory {} has no main.mim",
            path.display()
        )));
    }

    files.sort_by(|(a, _), (b, _)| a.cmp(b));

    solver.set_sources(sources.clone());

    // a file that failed to parse never registers its module, so solving the survivors just
    // cascades bogus not-found errors on top of the real one -- gate solving on a clean parse
    if errors.is_empty()
        && let Err(e) = solver.solve_all(files.iter().map(|(_, ast)| ast))
    {
        errors.push(e);
    }

    let stmts = files
        .into_iter()
        .flat_map(|(_, ast)| ast.unpack())
        .collect();

    let summary = SolveSummary {
        sources,
        errors,
        io_errors,
        lines_parsed,
        stmts,
        solver,
        intrinsics: library.intrinsics().iter().map(|(a, b)| (*a, *b)).collect(),
    };
    (vm, summary)
}

pub struct SolveSummary {
    pub sources: vm::Sources,
    pub errors: Vec<SolveError>,
    pub io_errors: Vec<std::io::Error>,
    pub lines_parsed: usize,
    pub stmts: Vec<Stmt>,
    pub solver: Solver,
    pub intrinsics: HashMap<NativeId, Intrinsic>,
}

impl SolveSummary {
    pub fn had_errors(&self) -> bool {
        !self.errors.is_empty() || !self.io_errors.is_empty()
    }

    pub fn into_compilation(
        self,
    ) -> (Vec<Stmt>, Solver, vm::Sources, HashMap<NativeId, Intrinsic>) {
        (self.stmts, self.solver, self.sources, self.intrinsics)
    }
}
