use std::path::{Path, PathBuf};

use api::Library;
use indexmap::IndexMap;
use lsp_types::{
    Contents, Diagnostic, DocumentHighlight, DocumentSymbol, Hover, InlayHint, InlayHintKind,
    Label, Location, MarkupContent, MarkupKind, Position, Range, Uri,
};
use parse::{NodeId, Stmt, StmtKind, Visitor, walk_stmts};
use shared::{FileId, Span, Ty};
use solve::{Resolutions, ResolvedDeclKind, components::DecId};

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

    /// Every ident in the project that resolves to the same declaration as the one at
    /// `position`, the declaration itself included when `with_declaration`.
    pub fn references(
        &self,
        path: &Path,
        position: Position,
        with_declaration: bool,
    ) -> Option<Vec<Location>> {
        let (resolutions, target) = self.dec_at(path, position)?;
        let declared_at = resolutions.decs[target].location;

        let mut locations = Vec::new();
        for (file_id, (path, file)) in self.files.iter().enumerate() {
            let Ok(uri) = Uri::from_file_path(path) else {
                continue;
            };
            for (id, span) in file.idents() {
                if resolutions.node_decs.get(&id) != Some(&target) {
                    continue;
                }
                let is_declaration = file_id == declared_at.file_id && span == declared_at.span;
                if is_declaration && !with_declaration {
                    continue;
                }
                if let Some(range) = file.range(span) {
                    locations.push(Location {
                        uri: uri.clone(),
                        range,
                    });
                }
            }
        }
        Some(locations)
    }

    /// Every use of the name at `position` within its own file.
    pub fn highlights(&self, path: &Path, position: Position) -> Option<Vec<DocumentHighlight>> {
        let (resolutions, target) = self.dec_at(path, position)?;
        let file = self.files.get(path)?;
        let highlights = file
            .idents()
            .into_iter()
            .filter(|(id, _)| resolutions.node_decs.get(id) == Some(&target))
            .filter_map(|(_, span)| {
                Some(DocumentHighlight {
                    range: file.range(span)?,
                    kind: None,
                })
            })
            .collect();
        Some(highlights)
    }

    /// The declaration the name at `position` resolves to.
    fn dec_at(&self, path: &Path, position: Position) -> Option<(&Resolutions, DecId)> {
        let resolutions = self.analysis.as_ref().ok()?;
        let file = self.files.get(path)?;
        let (id, _) = file.ast.node_at(file.offset(position)?)?;
        Some((resolutions, *resolutions.node_decs.get(&id)?))
    }

    /// A `: type` after every `let` in `range` that was written without an annotation.
    pub fn inlay_hints(&self, path: &Path, range: Range) -> Option<Vec<InlayHint>> {
        #[derive(Default)]
        struct Bindings(Vec<(NodeId, Span)>);

        impl Visitor for Bindings {
            fn stmt(&mut self, stmt: &Stmt) {
                let StmtKind::Let(binding) = stmt.kind() else {
                    return;
                };
                if binding.annotation.is_some() {
                    return;
                }
                if let Some(name) = binding.left.as_ident() {
                    self.0.push((name.id, name.location.span));
                }
            }
        }

        let resolutions = self.analysis.as_ref().ok()?;
        let file = self.files.get(path)?;
        let mut bindings = Bindings::default();
        walk_stmts(file.ast.stmts(), &mut bindings);

        let within = |position: &Position| {
            let at = (position.line, position.character);
            at >= (range.start.line, range.start.character)
                && at <= (range.end.line, range.end.character)
        };

        let hints = bindings
            .0
            .into_iter()
            .filter_map(|(id, span)| {
                let dec = *resolutions.node_decs.get(&id)?;
                let position = file.range(span)?.end;
                if !within(&position) {
                    return None;
                }
                Some(InlayHint {
                    position,
                    label: Label::String(format!(
                        ": {}",
                        resolutions.decs[dec].ty.display(resolutions)
                    )),
                    kind: Some(InlayHintKind::Type),
                    text_edits: None,
                    tooltip: None,
                    padding_left: None,
                    padding_right: None,
                    data: None,
                })
            })
            .collect();
        Some(hints)
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
