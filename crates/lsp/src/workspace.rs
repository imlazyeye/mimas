use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use api::Library;
use indexmap::{IndexMap, IndexSet};
use lsp_types::{Diagnostic, Uri};

use crate::{analysis::Analysis, host_api::std_library, project::Project};

/// Every project the editor has touched, by root directory, and the text of every open file.
pub struct Workspace {
    pub library: Library<()>,
    open: HashMap<PathBuf, String>,
    projects: HashMap<PathBuf, Project>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            library: std_library(),
            open: HashMap::new(),
            projects: HashMap::new(),
        }
    }
}

impl Workspace {
    /// Fetches the Project housed at a given path, if any.
    pub fn project(&self, path: &Path) -> Option<&Project> {
        self.projects
            .values()
            .find(|project| project.analysis(path).is_some())
    }

    /// Fetches the analysis for a project at the given path, if any
    pub fn analysis(&self, path: &Path) -> Option<&Analysis> {
        self.project(path)?.analysis(path)
    }

    /// Takes an open file's text (none once closed) and rebuilds the project it belongs to, whole,
    /// along with any other that held it. Gives back the diagnostics to publish, an empty list
    /// clearing a file that left.
    pub fn update(&mut self, path: &Path, text: Option<String>) -> Vec<(Uri, Vec<Diagnostic>)> {
        match text {
            Some(text) => self.open.insert(path.to_path_buf(), text),
            None => self.open.remove(path),
        };
        let mut roots: IndexSet<PathBuf> = self
            .projects
            .iter()
            .filter(|(_, project)| project.files().any(|file| file == path))
            .map(|(root, _)| root.clone())
            .collect();
        roots.insert(self.root_of(path));

        let mut cleared: Vec<Uri> = Vec::new();
        let mut published: IndexMap<Uri, Vec<Diagnostic>> = IndexMap::new();
        for root in roots {
            // the root's old project goes, and any that sat below it: their files are its now
            self.projects.retain(|key, project| {
                let stale = key.starts_with(&root);
                if stale {
                    cleared.extend(
                        project
                            .files()
                            .filter_map(|file| Uri::from_file_path(file).ok()),
                    );
                }
                !stale
            });
            let files = self.read(&root, true).collect();
            let project = Project::load(&root, files, &self.library);
            published.extend(project.diagnostics());
            if project.files().next().is_some() {
                self.projects.insert(root, project);
            }
        }
        for uri in cleared {
            published.entry(uri).or_default();
        }
        published.into_iter().collect()
    }

    /// Every `.mim` in `root` (or below it too), open or on disk, with the editor's text winning.
    fn read(&self, root: &Path, recursive: bool) -> impl Iterator<Item = (PathBuf, String)> {
        let mut paths = solve::mim_files(root, recursive).0;
        paths.extend(
            self.open
                .keys()
                .filter(|open| match recursive {
                    true => open.starts_with(root),
                    false => solve::dir_of(open) == root,
                })
                .cloned(),
        );
        paths.sort();
        paths.dedup();
        paths.into_iter().filter_map(|path| {
            let text = match self.open.get(&path) {
                Some(text) => text.clone(),
                None => std::fs::read_to_string(&path).ok()?,
            };
            Some((path, text))
        })
    }

    /// The project a file belongs to the nearest directory at or above it that holds a script,
    /// or its own when none does. The walk stops at a repository root.
    fn root_of(&self, path: &Path) -> PathBuf {
        let dir = solve::dir_of(path);
        if self
            .open
            .get(path)
            .is_some_and(|text| !parse::lex::is_module(text))
        {
            return dir.to_path_buf();
        }
        for ancestor in dir.ancestors() {
            let has_script = self
                .read(ancestor, false)
                .any(|(_, text)| !parse::lex::is_module(&text));
            if has_script {
                return ancestor.to_path_buf();
            }
            if ancestor.join(".git").exists() {
                break;
            }
        }
        dir.to_path_buf()
    }
}
