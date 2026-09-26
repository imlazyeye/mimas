use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

use api::Library;
use indexmap::{IndexMap, IndexSet};
use lsp_types::{Location, Position, TextEdit, Uri, WorkspaceEdit};

use crate::analysis::Analysis;

/// The files under one root, solved the way a host runs them: the modules on their own, and each
/// script with every module. A script sees every module and nothing of the other scripts.
pub struct Solved {
    modules: Analysis,
    scripts: IndexMap<PathBuf, Analysis>,
}

impl Solved {
    pub fn load(files: Vec<(PathBuf, String)>, library: Library<()>) -> Self {
        let library = Rc::new(library);
        let (modules, scripts): (Vec<_>, Vec<_>) = files
            .into_iter()
            .partition(|(_, text)| parse::lex::is_module(text));
        // the script goes last, so a conflict between it and a module is reported in the script
        let scripts = scripts
            .into_iter()
            .map(|script| {
                let path = script.0.clone();
                let files = modules.iter().cloned().chain([script]).collect();
                (path, Analysis::load(files, library.clone()))
            })
            .collect();
        Self {
            modules: Analysis::load(modules, library),
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

    /// Every use of the name at `position`: in the modules and every script when it's declared
    /// in a module, or in its own file's analysis otherwise. A script that doesn't check has
    /// nothing to add.
    pub fn references(
        &self,
        path: &Path,
        position: Position,
        with_declaration: bool,
    ) -> Option<Vec<Location>> {
        let (path, position, analyses) = self.everywhere(path, position)?;
        let locations: IndexSet<Location> = analyses
            .into_iter()
            .flat_map(|analysis| analysis.references(&path, position, with_declaration))
            .flatten()
            .collect();
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
            let edit = analysis.rename(&path, position, new_name)?;
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
        // with no declaration to follow (a builtin, or a file that doesn't check), the file's
        // own analysis answers alone
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
