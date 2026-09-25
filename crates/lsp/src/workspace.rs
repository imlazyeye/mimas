use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    rc::Rc,
};

use lsp_types::{Diagnostic, MessageType, Uri};

use crate::{
    analysis::Analysis,
    host_api::{HostApis, Source},
    project::Project,
};

/// The projects of the files open in the editor, by root directory, and the text of every open
/// file.
pub struct Workspace {
    hosts: HostApis,
    open: HashMap<PathBuf, String>,
    projects: HashMap<PathBuf, Project>,
    published: HashMap<PathBuf, Vec<Diagnostic>>,
}

impl Workspace {
    pub fn new(source: Source) -> Self {
        Self {
            hosts: HostApis::new(source),
            open: HashMap::new(),
            projects: HashMap::new(),
            published: HashMap::new(),
        }
    }

    /// The project a file belongs to: the deepest loaded one that holds it.
    pub fn project(&self, path: &Path) -> Option<&Project> {
        self.projects
            .iter()
            .filter(|(_, project)| project.analysis(path).is_some())
            .max_by_key(|(root, _)| *root)
            .map(|(_, project)| project)
    }

    /// The analysis that answers for an open file.
    pub fn analysis(&self, path: &Path) -> Option<&Analysis> {
        self.project(path)?.analysis(path)
    }

    /// Takes an open file's text (none once closed) and rebuilds its project, along with any other
    /// that holds it or whose host API changed. A project no open file belongs to goes. Gives back
    /// the diagnostics that changed, an empty list clearing a file, and what the user should hear
    /// about.
    pub fn update(
        &mut self,
        path: &Path,
        text: Option<String>,
    ) -> (Vec<(Uri, Vec<Diagnostic>)>, Vec<(MessageType, String)>) {
        match text {
            Some(text) => self.open.insert(path.to_path_buf(), text),
            None => self.open.remove(path),
        };
        // any file's text can decide whether its folder holds a script, so every root can move
        let dirs: HashSet<&Path> = self.open.keys().map(|file| solve::dir_of(file)).collect();
        let roots: HashMap<PathBuf, Option<PathBuf>> =
            dirs.into_iter().map(|dir| self.root_of(dir)).collect();
        self.projects.retain(|root, _| roots.contains_key(root));
        for (root, package) in roots {
            let library = self.hosts.library(package.as_deref());
            let fresh = self.projects.get(&root).is_some_and(|project| {
                Rc::ptr_eq(&project.library, &library) && !project.files().any(|file| file == path)
            });
            if !fresh {
                let project = Project::load(self.read(&root, true).collect(), library);
                self.projects.insert(root, project);
            }
        }
        return (publish(self), std::mem::take(&mut self.hosts.messages));

        // when projects nest, the deepest that holds a file answers for it
        fn publish(workspace: &mut Workspace) -> Vec<(Uri, Vec<Diagnostic>)> {
            let mut projects: Vec<_> = workspace.projects.iter().collect();
            projects.sort_by_key(|(root, _)| *root);
            let mut current = HashMap::new();
            for (_, project) in projects {
                current.extend(project.diagnostics());
            }
            let cleared = workspace
                .published
                .keys()
                .filter(|file| !current.contains_key(*file))
                .map(|file| (file.clone(), Vec::new()));
            let changed: Vec<_> = current
                .iter()
                .filter(|(file, diagnostics)| workspace.published.get(*file) != Some(diagnostics))
                .map(|(file, diagnostics)| (file.clone(), diagnostics.clone()))
                .chain(cleared)
                .collect();
            workspace.published = current;
            changed
                .into_iter()
                .filter_map(|(file, diagnostics)| {
                    Some((Uri::from_file_path(file).ok()?, diagnostics))
                })
                .collect()
        }
    }

    /// The root of the project a file in `dir` belongs to, with the cargo package it sits in.
    /// Inside a package, the root is the highest folder in the package that holds a script, the
    /// way a host loads its whole scripts folder. Outside one, it's the nearest folder at or above
    /// `dir` that holds a script, the way `mimas run` does. It's `dir` itself when there's no such
    /// folder.
    pub fn root_of(&self, dir: &Path) -> (PathBuf, Option<PathBuf>) {
        let mut nearest = None;
        let mut highest = None;
        let mut package = None;
        for ancestor in dir.ancestors() {
            let holds_script = self
                .read(ancestor, false)
                .any(|(_, text)| !parse::lex::is_module(&text));
            if holds_script {
                nearest.get_or_insert(ancestor);
                highest = Some(ancestor);
            }
            if solve::is_package(ancestor) {
                package = Some(ancestor);
                break;
            }
            if ancestor.join(".git").exists() {
                break;
            }
        }
        let root = match package {
            Some(_) => highest,
            None => nearest,
        }
        .unwrap_or(dir);
        (root.to_path_buf(), package.map(Path::to_path_buf))
    }

    /// Every `.mim` in `root` (or below it too), open or on disk, with the editor's text winning.
    /// Like [`solve::mim_files`], it leaves out what sits past a boundary below `root`.
    fn read(&self, root: &Path, recursive: bool) -> impl Iterator<Item = (PathBuf, String)> {
        let mut paths = solve::mim_files(root, recursive).0;
        paths.extend(
            self.open
                .keys()
                .filter(|open| match recursive {
                    true => {
                        open.starts_with(root)
                            && !solve::dir_of(open)
                                .ancestors()
                                .take_while(|dir| *dir != root)
                                .any(solve::is_boundary)
                    }
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
}
