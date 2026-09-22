use std::path::{Path, PathBuf};

use api::Library;
use indexmap::IndexMap;
use lsp_types::{
    Contents, Diagnostic, DocumentSymbol, Hover, Location, MarkupContent, MarkupKind, Position, Uri,
};
use shared::{FileId, Ty};
use solve::{Resolutions, ResolvedDeclKind};

use crate::source_file::SourceFile;

/// A set of files solved together, the same way `mimas check` treats a directory. A file with no
/// `main.mim` above it is a project of one.
pub struct Project {
    /// In file id order.
    pub files: IndexMap<PathBuf, SourceFile>,
    /// The errors when any file failed to parse or the solve failed.
    pub analysis: Result<Resolutions, Vec<shared::Error>>,
}

impl Project {
    pub fn load(files: Vec<(PathBuf, String)>, library: &Library<()>) -> Self {
        // the full path, like the cli (`module @` takes its name from it)
        let names: Vec<String> = files
            .iter()
            .map(|(path, _)| path.to_string_lossy().into_owned())
            .collect();
        let texts = files.iter().map(|(_, text)| text.as_str());
        let loaded = solve::load_files(names.iter().map(String::as_str).zip(texts), library);
        let files = files
            .into_iter()
            .zip(loaded.asts)
            .map(|((path, text), ast)| (path, SourceFile::new(&text, ast)))
            .collect();
        let analysis = if loaded.errors.is_empty() {
            Ok(Resolutions::from(loaded.solver))
        } else {
            Err(loaded.errors)
        };
        Self { files, analysis }
    }

    pub fn hover(&self, path: &Path, position: Position) -> Option<Hover> {
        let resolutions = self.analysis.as_ref().ok()?;
        let file = self.files.get(path)?;
        let offset = file.offset(position)?;
        let (id, span) = file.ast.node_at(offset)?;
        let dec_id = resolutions.node_decs.get(&id).copied();
        let dec = dec_id.map(|dec| &resolutions.decs[dec]);
        let ty = resolutions
            .node_tys
            .get(&id)
            .or_else(|| dec.map(|dec| &dec.ty))?;

        let text = match dec_id.zip(dec) {
            // a type named in a path or literal has no dec of its own, just the adt
            None => match ty {
                Ty::Adt(aid) => resolutions.adt_path(*aid),
                ty => ty.display(resolutions),
            },
            Some((dec_id, dec)) => {
                let (owner, decl) = match (&dec.kind, ty) {
                    (ResolvedDeclKind::Item { takes_self, .. }, Ty::Fn(header)) => (
                        resolutions.owner_path(dec_id),
                        header.signature(&dec.name, *takes_self, resolutions),
                    ),
                    (ResolvedDeclKind::Adt(aid), _) => (String::new(), resolutions.adt_path(*aid)),
                    (ResolvedDeclKind::Constant(_), _) => (
                        resolutions.owner_path(dec_id),
                        format!("const {}: {}", dec.name, ty.display(resolutions)),
                    ),
                    _ => (
                        String::new(),
                        format!("{}: {}", dec.name, ty.display(resolutions)),
                    ),
                };
                if owner.is_empty() {
                    decl
                } else {
                    format!("{owner}\n{decl}")
                }
            }
        };

        Some(Hover {
            contents: Contents::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("```mimas\n{text}\n```"),
            }),
            range: file.range(span),
        })
    }

    /// Where the name at `position` was declared.
    pub fn definition(&self, path: &Path, position: Position) -> Option<Location> {
        let resolutions = self.analysis.as_ref().ok()?;
        let file = self.files.get(path)?;
        let (id, _) = file.ast.node_at(file.offset(position)?)?;
        let dec = &resolutions.decs[*resolutions.node_decs.get(&id)?];
        let span = dec.location.span;
        // natives, builtins, and module segments are declared with no source behind them
        if span.is_synthetic() || span.is_empty() {
            return None;
        }
        let (target_path, target) = self.files.get_index(dec.location.file_id)?;
        Some(Location {
            uri: Uri::from_file_path(target_path).ok()?,
            range: target.range(span)?,
        })
    }

    /// The outline of one file, which needs no analysis.
    pub fn symbols(&self, path: &Path) -> Option<Vec<DocumentSymbol>> {
        Some(self.files.get(path)?.symbols())
    }

    /// Every file's diagnostics (an empty list clears what the editor last showed).
    pub fn diagnostics(&self) -> Vec<(Uri, Vec<Diagnostic>)> {
        let mut per_file: Vec<Vec<Diagnostic>> = self.files.iter().map(|_| Vec::new()).collect();
        for error in self.analysis.as_ref().err().into_iter().flatten() {
            let Some(file_id) = self.file_of(error).or((!per_file.is_empty()).then_some(0)) else {
                continue;
            };
            per_file[file_id].push(self.files[file_id].diagnostic(error));
        }
        self.files
            .keys()
            .zip(per_file)
            .filter_map(|(path, diagnostics)| Some((Uri::from_file_path(path).ok()?, diagnostics)))
            .collect()
    }

    /// Which file an error points into, by the name its source was registered under.
    fn file_of(&self, error: &shared::Error) -> Option<FileId> {
        let label = error.labels()?.next()?;
        let contents = error.source_code()?.read_span(label.inner(), 0, 0).ok()?;
        self.files.get_index_of(Path::new(contents.name()?))
    }
}
