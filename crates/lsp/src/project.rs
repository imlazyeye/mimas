use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

use api::Library;
use indexmap::{IndexMap, IndexSet};
use lsp_types::{Diagnostic, DiagnosticSeverity, Location, Position, TextEdit, Uri, WorkspaceEdit};

use crate::{analysis::Analysis, source_file::SourceFile};

/// A directory as a library. Modules are solved once and each script solved on top of them. A
/// module's file is shared by every analysis that holds it.
pub struct Project {
    modules: Analysis,
    scripts: IndexMap<PathBuf, Analysis>,
    out_of_place_scripts: Vec<PathBuf>,
}

impl Project {
    pub fn load(root: &Path, files: Vec<(PathBuf, String)>, library: &Library<()>) -> Self {
        let solve::Directory {
            module_files,
            modules,
            scripts,
            out_of_place_scripts,
        } = solve::Directory::load(&files, root, library);
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
            modules: Analysis::new(module_files, errors, solver),
            scripts,
            out_of_place_scripts: out_of_place_scripts
                .into_iter()
                .map(Path::to_path_buf)
                .collect(),
        }
    }

    /// Every file the project holds.
    pub fn files(&self) -> impl Iterator<Item = &PathBuf> {
        self.modules
            .files
            .keys()
            .chain(self.scripts.keys())
            .chain(&self.out_of_place_scripts)
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

    /// Every file's diagnostics: the modules' once, each script's own, and one for each script
    /// out of place.
    pub fn diagnostics(&self) -> Vec<(Uri, Vec<Diagnostic>)> {
        let of = |analysis: &Analysis, path: &PathBuf| {
            Some((Uri::from_file_path(path).ok()?, analysis.diagnostics(path)))
        };
        let modules = self
            .modules
            .files
            .keys()
            .filter_map(|path| of(&self.modules, path));
        let scripts = self
            .scripts
            .iter()
            .filter_map(|(path, script)| of(script, path));
        let nested = self.out_of_place_scripts.iter().filter_map(|path| {
            let out_of_place = Diagnostic {
                severity: Some(DiagnosticSeverity::Error),
                source: Some("mimas".to_owned()),
                message: "a script has to sit at the top of its project".into(),
                ..Default::default()
            };
            Some((Uri::from_file_path(path).ok()?, vec![out_of_place]))
        });
        modules.chain(scripts).chain(nested).collect()
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
        library: &Library<()>,
    ) -> Result<WorkspaceEdit, String> {
        let (path, position, analyses) = self
            .everywhere(path, position)
            .ok_or("that file isn't part of a project")?;
        let mut changes: HashMap<Uri, IndexSet<TextEdit>> = HashMap::new();
        for analysis in analyses {
            let edit = analysis.rename(&path, position, new_name, library)?;
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
