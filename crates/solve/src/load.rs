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

/// A directory as a library: its modules, solved together once, and its scripts, each solved on
/// top of them with [`Modules::load`] like every script is solved on top of the std library.
///
/// - my_project
///   - module_a.mim
///   - script_a.mim
///   - enemies
///     - module_b.mim
///     - script_b.mim
///
/// `my_project` is one library of `module_a` and `module_b`. `script_a` and `script_b` each see
/// all of it and nothing of each other.
pub struct Directory<'a> {
    pub module_files: Vec<&'a (PathBuf, String)>,
    pub modules: Modules,
    pub scripts: Vec<&'a (PathBuf, String)>,
}

impl<'a> Directory<'a> {
    /// Sorts a project's files (each a path and its text) into modules and scripts, and solves the
    /// modules against `library`.
    pub fn load(files: &'a [(PathBuf, String)], library: &Library<()>) -> Self {
        let (module_files, scripts): (Vec<_>, Vec<_>) = files
            .iter()
            .partition(|file| parse::lex::is_module(&file.1));
        let modules = Modules::from_files(
            module_files
                .iter()
                .map(|(path, text)| (path, text.as_str())),
            library,
        );
        Self {
            module_files,
            modules,
            scripts,
        }
    }
}

/// Every `.mim` file under `root` (or only directly in it, unless `recursive`), sorted by path,
/// with whatever the walk couldn't read. The walk leaves out any directory below `root` that
/// [`is_boundary`].
pub fn mim_files(root: &Path, recursive: bool) -> (Vec<PathBuf>, Vec<std::io::Error>) {
    let mut files = Vec::new();
    let mut errors = Vec::new();
    let depth = if recursive { usize::MAX } else { 1 };
    let walk = WalkDir::new(root)
        .max_depth(depth)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0 || !entry.file_type().is_dir() || !is_boundary(entry.path())
        });
    for entry in walk {
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

/// Whether a project stops short of `dir`, which is another cargo package or a cargo target dir.
pub fn is_boundary(dir: &Path) -> bool {
    is_package(dir) || dir.join("CACHEDIR.TAG").is_file()
}

/// Whether `dir` holds a cargo package, not just a workspace.
pub fn is_package(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("Cargo.toml"))
        .is_ok_and(|toml| toml.lines().any(|line| line.trim() == "[package]"))
}

/// A file's directory, `.` for a bare name, so a path typed and a path walked compare alike.
pub fn dir_of(path: &Path) -> &Path {
    match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    }
}
