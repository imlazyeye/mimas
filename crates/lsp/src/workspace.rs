use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use lsp_types::{Diagnostic, DiagnosticSeverity, Uri};

use crate::{analysis::Analysis, host_api::HostApi, project::Project};

/// The text of every open file, and the projects they belong to, by root. Projects never overlap.
pub struct Workspace {
    host_api: HostApi,
    open: HashMap<PathBuf, String>,
    projects: HashMap<PathBuf, Project>,
}

impl Workspace {
    pub fn new(host_api: HostApi) -> Self {
        Self {
            host_api,
            open: HashMap::new(),
            projects: HashMap::new(),
        }
    }

    /// The project a file belongs to, while any of its files is open.
    pub fn project(&self, path: &Path) -> Option<&Project> {
        self.projects.get(root_of(path).0)
    }

    /// The analysis that answers for an open file.
    pub fn analysis(&self, path: &Path) -> Option<&Analysis> {
        self.project(path)?.analysis(path)
    }

    /// Takes an open file's text (none once closed) and rebuilds its project from scratch, or
    /// drops it once none of its files are open. Gives back the diagnostics of every file the
    /// project held or holds, an empty list clearing one it lost.
    pub fn update(&mut self, path: &Path, text: Option<String>) -> Vec<(Uri, Vec<Diagnostic>)> {
        match text {
            Some(text) => self.open.insert(path.to_path_buf(), text),
            None => self.open.remove(path),
        };
        let (root, recursive, package) = root_of(path);
        let old = self.projects.remove(root);
        let problem = if self.open.keys().any(|file| root_of(file).0 == root) {
            // every `.mim` in the project, with the editor's text winning
            let files = solve::mim_files(root, recursive)
                .0
                .into_iter()
                .filter_map(|path| {
                    let text = match self.open.get(&path) {
                        Some(text) => text.clone(),
                        None => std::fs::read_to_string(&path).ok()?,
                    };
                    Some((path, text))
                })
                .collect();
            let (library, problem) = self.host_api.library(package);
            self.projects
                .insert(root.to_path_buf(), Project::load(files, library));
            problem
        } else {
            None
        };

        // a host API problem shows at the top of every file it leaves checked against std alone
        let note = problem.map(|message| Diagnostic {
            severity: Some(DiagnosticSeverity::Warning),
            source: Some("mimas".to_owned()),
            message: message.into(),
            ..Default::default()
        });
        let new = self.projects.get(root);
        let files: HashSet<&PathBuf> = old.iter().chain(new).flat_map(Project::files).collect();
        files
            .into_iter()
            .filter_map(|file| {
                let diagnostics = match new.and_then(|new| new.analysis(file)) {
                    Some(analysis) => {
                        let errors = analysis.diagnostics(file).into_iter();
                        errors.chain(note.clone()).collect()
                    }
                    None => Vec::new(),
                };
                Some((Uri::from_file_path(file).ok()?, diagnostics))
            })
            .collect()
    }
}

/// Where a file's project sits: the highest folder holding a `.mim` in the cargo package or
/// repository the file is in, with everything below it, the way a host loads its whole scripts
/// folder. Outside both, it's the file's own folder alone. Gives back that folder, whether the
/// project takes in the folders below it, and the package.
pub(crate) fn root_of(file: &Path) -> (&Path, bool, Option<&Path>) {
    let dir = solve::dir_of(file);
    let mut highest = dir;
    for ancestor in dir.ancestors() {
        if !solve::mim_files(ancestor, false).0.is_empty() {
            highest = ancestor;
        }
        if solve::is_package(ancestor) {
            return (highest, true, Some(ancestor));
        }
        if ancestor.join(".git").exists() {
            return (highest, true, None);
        }
    }
    (dir, false, None)
}
