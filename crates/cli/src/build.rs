use api::{Intrinsic, NativeId};
use parse::{Ast, Stmt};
use solve::{Error as SolveError, Solver};
use std::{collections::HashMap, path::Path};

pub fn solve(path: &Path) -> (vm::Vm, SolveSummary) {
    let mut vm = vm::Vm::new();
    let library = vm.install_library(library::std);

    // a file is taken as given, whatever its extension. a directory is walked for `.mim`s
    let (paths, mut io_errors) = if path.is_file() {
        (vec![path.to_path_buf()], vec![])
    } else {
        solve::mim_files(path)
    };
    let mut files: Vec<(String, String)> = vec![];
    let mut lines_parsed = 0;
    let mut has_main = false;
    for file_path in paths {
        let mimas = match std::fs::read_to_string(&file_path) {
            Ok(mimas) => mimas,
            Err(e) => {
                io_errors.push(std::io::Error::new(
                    e.kind(),
                    format!("{}: {e}", file_path.display()),
                ));
                continue;
            }
        };
        if file_path.file_stem().and_then(|n| n.to_str()) == Some("main") {
            has_main = true;
        }
        lines_parsed += mimas.lines().count();
        // full path, not the stem: this is the name parse diagnostics render, and solve/runtime
        // already use the full path -- `module @` stem-ifies it on its own
        files.push((file_path.to_str().unwrap().to_string(), mimas));
    }

    // directory mode requires main.mim as the entry. single-file mode is whatever the user pointed
    // at. todo, enforce that non-main files in directory mode are pure module declarations.
    if path.is_dir() && !has_main {
        io_errors.push(std::io::Error::other(format!(
            "directory {} has no main.mim",
            path.display()
        )));
    }

    let loaded = solve::load_files(
        files
            .iter()
            .map(|(name, text)| (name.as_str(), text.as_str())),
        &library,
    );
    let stmts = loaded.asts.into_iter().flat_map(Ast::unpack).collect();

    let summary = SolveSummary {
        sources: loaded.sources,
        errors: loaded.errors,
        io_errors,
        lines_parsed,
        stmts,
        solver: loaded.solver,
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
