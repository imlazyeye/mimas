use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

use api::Library;
use indexmap::{IndexMap, IndexSet};
use lsp_types::{Diagnostic, Location, Position, TextEdit, Uri, WorkspaceEdit};

use crate::{analysis::Analysis, source_file::SourceFile};

/// A directory as a library. Modules are solved once and each script solved on top of them. A
/// module's file is shared by every analysis that holds it.
pub struct Project {
    pub library: Rc<Library<()>>,
    modules: Analysis,
    scripts: IndexMap<PathBuf, Analysis>,
}

impl Project {
    pub fn load(files: Vec<(PathBuf, String)>, library: Rc<Library<()>>) -> Self {
        let solve::Directory {
            module_files,
            modules,
            scripts,
        } = solve::Directory::load(&files, &library);
        let scripts: Vec<_> = scripts
            .into_iter()
            .map(|file| (file, modules.load([(&file.0, file.1.as_str())])))
            .collect();
        let solve::Modules {
            asts,
            errors,
            solver,
            ..
        } = modules;
        let module_files: IndexMap<PathBuf, Rc<SourceFile>> = module_files
            .iter()
            .zip(asts)
            .map(|((path, text), ast)| (path.clone(), Rc::new(SourceFile::new(text.clone(), ast))))
            .collect();
        let scripts = scripts
            .into_iter()
            .map(|((path, text), script)| {
                let solve::Modules {
                    mut asts,
                    errors,
                    solver,
                    ..
                } = script;
                let mut files = module_files.clone();
                let file = Rc::new(SourceFile::new(text.clone(), asts.remove(0)));
                files.insert(path.clone(), file);
                (path.clone(), Analysis::new(files, errors, solver))
            })
            .collect();
        Self {
            library,
            modules: Analysis::new(module_files, errors, solver),
            scripts,
        }
    }

    /// Every file the project holds.
    pub fn files(&self) -> impl Iterator<Item = &PathBuf> {
        self.modules.files.keys().chain(self.scripts.keys())
    }

    /// The analysis that answers for `path`: a script's own, or the modules' for a module.
    pub fn analysis(&self, path: &Path) -> Option<&Analysis> {
        self.scripts.get(path).or_else(|| {
            self.modules
                .files
                .contains_key(path)
                .then_some(&self.modules)
        })
    }

    /// Every file's diagnostics: the modules' once, and each script's own.
    pub fn diagnostics(&self) -> Vec<(PathBuf, Vec<Diagnostic>)> {
        let modules = self
            .modules
            .files
            .keys()
            .map(|path| (path.clone(), self.modules.diagnostics(path)));
        let scripts = self
            .scripts
            .iter()
            .map(|(path, script)| (path.clone(), script.diagnostics(path)));
        modules.chain(scripts).collect()
    }

    /// Every use of the name at `position`: in the modules and every script when it's declared
    /// in a module, or in its own file's analysis otherwise.
    pub fn references(
        &self,
        path: &Path,
        position: Position,
        with_declaration: bool,
    ) -> Option<Vec<Location>> {
        let (path, position, analyses) = self.everywhere(path, position)?;
        let mut locations = IndexSet::new();
        for analysis in analyses {
            locations.extend(analysis.references(&path, position, with_declaration)?);
        }
        Some(locations.into_iter().collect())
    }

    /// Renames the name at `position` everywhere [`Self::references`] would find it.
    pub fn rename(
        &self,
        path: &Path,
        position: Position,
        new_name: &str,
    ) -> Result<WorkspaceEdit, String> {
        let (path, position, analyses) = self
            .everywhere(path, position)
            .ok_or("that file isn't part of a project")?;
        let mut changes: HashMap<Uri, IndexSet<TextEdit>> = HashMap::new();
        for analysis in analyses {
            let edit = analysis.rename(&path, position, new_name, &self.library)?;
            for (uri, edits) in edit.changes.into_iter().flatten() {
                changes.entry(uri).or_default().extend(edits);
            }
        }
        Ok(WorkspaceEdit {
            changes: Some(
                changes
                    .into_iter()
                    .map(|(uri, edits)| (uri, edits.into_iter().collect()))
                    .collect(),
            ),
            ..Default::default()
        })
    }

    /// Where the name at `position` is declared, and every analysis that can see it.
    fn everywhere(
        &self,
        path: &Path,
        position: Position,
    ) -> Option<(PathBuf, Position, Vec<&Analysis>)> {
        let analysis = self.analysis(path)?;
        // a builtin has no declaration to follow, and the analysis's own answer says why
        let Some((declared, at)) = analysis
            .definition(path, position)
            .and_then(|location| Some((location.uri.to_file_path().ok()?, location.range.start)))
        else {
            return Some((path.to_path_buf(), position, vec![analysis]));
        };
        let analyses = if self.modules.files.contains_key(&declared) {
            std::iter::once(&self.modules)
                .chain(self.scripts.values())
                .collect()
        } else {
            vec![self.analysis(&declared)?]
        };
        Some((declared, at, analyses))
    }
}
