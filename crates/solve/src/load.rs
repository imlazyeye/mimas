use std::{collections::HashMap, path::Path, sync::Arc};

use api::Library;
use miette::NamedSource;
use parse::{Ast, Parser, lex::Lexer};
use shared::FileId;

use crate::{Error, Solver};

/// Files parsed and solved together, on top of the library or of other modules. File ids follow
/// the order the files were given in, after the ones below them. `asts` holds only these files,
/// while `sources` and `solver` cover the ones below too.
pub struct Modules {
    pub asts: Vec<Ast>,
    pub sources: HashMap<FileId, NamedSource<Arc<str>>>,
    pub errors: Vec<Error>,
    pub solver: Solver,
}

impl Modules {
    /// Parses and solves `files` (each a path and its text) against `library`.
    pub fn from_files<'a>(
        files: impl IntoIterator<Item = (impl AsRef<Path>, &'a str)>,
        library: &Library<()>,
    ) -> Self {
        let mut solver = Solver::new();
        solver.install_library(library);
        let library = Self {
            asts: Vec::new(),
            sources: HashMap::new(),
            errors: Vec::new(),
            solver,
        };
        library.load(files)
    }

    /// Parses and solves `files` on top of these modules, the way these were solved on top of the
    /// library. Nothing is solved when either side has errors.
    pub fn load<'a>(&self, files: impl IntoIterator<Item = (impl AsRef<Path>, &'a str)>) -> Self {
        let mut asts = Vec::new();
        let mut sources = self.sources.clone();
        let mut errors = Vec::new();
        for (file_id, (path, text)) in (self.sources.len()..).zip(files) {
            let name = path.as_ref().to_string_lossy();
            sources.insert(file_id, NamedSource::new(&name, Arc::from(text)));
            let (ast, parse_errors) =
                Parser::new(Lexer::new(text, file_id, name.into_owned())).into_ast();
            errors.extend(parse_errors);
            asts.push(ast);
        }

        let mut solver = self.solver.clone();
        solver.set_sources(sources.clone());
        if self.errors.is_empty()
            && errors.is_empty()
            && let Err(error) = solver.solve_all(&asts)
        {
            errors.push(error);
        }

        Self {
            asts,
            sources,
            errors,
            solver,
        }
    }
}
