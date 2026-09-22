use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use api::Library;
use miette::NamedSource;
use parse::{Ast, Parser, lex::Lexer};
use shared::FileId;
use walkdir::WalkDir;

use crate::{Error, Solver};

/// The files of one program, parsed and solved together. File ids follow the order the files
/// were given in.
pub struct Loaded {
    pub asts: Vec<Ast>,
    pub sources: HashMap<FileId, NamedSource<Arc<str>>>,
    pub errors: Vec<Error>,
    pub solver: Solver,
}

/// Parses and solves `files` (each a name and its text) against `library`. The solver can't
/// take poison yet, so a parse error in any file skips the solve.
pub fn load_files<'a>(
    files: impl IntoIterator<Item = (&'a str, &'a str)>,
    library: &Library<()>,
) -> Loaded {
    let mut asts = Vec::new();
    let mut sources = HashMap::new();
    let mut errors = Vec::new();
    for (file_id, (name, text)) in files.into_iter().enumerate() {
        sources.insert(file_id, NamedSource::new(name, Arc::from(text)));
        let (ast, parse_errors) =
            Parser::new(Lexer::new(text, file_id, name.to_owned())).into_ast();
        errors.extend(parse_errors);
        asts.push(ast);
    }

    let mut solver = Solver::new();
    solver.install_library(library);
    solver.set_sources(sources.clone());
    if errors.is_empty()
        && let Err(error) = solver.solve_all(&asts)
    {
        errors.push(error);
    }

    Loaded {
        asts,
        sources,
        errors,
        solver,
    }
}

/// Every `.mim` file under `root`, sorted by path, with whatever the walk couldn't read.
pub fn mim_files(root: &Path) -> (Vec<PathBuf>, Vec<std::io::Error>) {
    let mut files = Vec::new();
    let mut errors = Vec::new();
    for entry in WalkDir::new(root) {
        match entry {
            Ok(entry) => {
                let is_mim = entry.file_type().is_file()
                    && entry.path().extension().and_then(|e| e.to_str()) == Some("mim");
                if is_mim {
                    files.push(entry.into_path());
                }
            }
            Err(e) => errors.push(std::io::Error::other(format!("{}: {e}", root.display()))),
        }
    }
    files.sort();
    (files, errors)
}
