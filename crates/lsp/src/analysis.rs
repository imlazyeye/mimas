use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

use api::{ApiEntry, Library, NativeId};
use indexmap::IndexMap;
use lsp_types::*;
use parse::{
    NodeId, Stmt, StmtKind, Visitor,
    lex::{Lexer, TokKind},
    walk_stmts,
};
use shared::{AdtId, FileId, Span, Ty};
use solve::{Resolutions, ResolvedDeclKind, components::DecId};

use crate::source_file::SourceFile;

/// Files solved together as one program, and the language features over them: the modules of a
/// project alone, or a script with every module.
pub struct Analysis {
    /// In file id order.
    pub files: IndexMap<PathBuf, SourceFile>,
    /// The errors when any file failed to parse or the solve failed.
    resolutions: Result<Resolutions, Vec<shared::Error>>,
    /// The host API the files were solved against.
    library: Rc<Library<()>>,
}

impl Analysis {
    /// Solves `files` together, as one program.
    pub fn load(files: Vec<(PathBuf, String)>, library: Rc<Library<()>>) -> Self {
        let loaded = solve::Modules::from_files(
            files.iter().map(|(path, text)| (path, text.as_str())),
            &library,
        );
        let files = files
            .into_iter()
            .zip(loaded.asts)
            .map(|((path, text), ast)| (path, SourceFile::new(text, ast)))
            .collect();
        let resolutions = match loaded.errors.is_empty() {
            true => Ok(Resolutions::from(loaded.solver)),
            false => Err(loaded.errors),
        };
        Self {
            files,
            resolutions,
            library,
        }
    }

    pub fn hover(&self, path: &Path, position: Position) -> Option<Hover> {
        let library = &self.library;
        let resolutions = self.resolutions.as_ref().ok()?;
        let file = self.files.get(path)?;
        let offset = file.offset(position)?;
        let (id, span) = file.ast.node_at(offset)?;
        let dec_id = resolutions.node_decs.get(&id).copied();
        let ty = resolutions
            .node_tys
            .get(&id)
            .or_else(|| dec_id.map(|dec| &resolutions.decs[dec].ty))?;

        let text = match dec_id {
            // a type named in a path or literal has no dec of its own, just the adt
            None => match ty {
                Ty::Adt(aid) => resolutions.adt_path(*aid),
                ty => ty.display(resolutions),
            },
            Some(dec_id) => {
                let dec = &resolutions.decs[dec_id];
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
        let adt = |aid: AdtId| library.adts().iter().find(|adt| adt.adt_id == aid);
        let native_doc = |native: NativeId| {
            let (_, entry) = library.natives().find(|(id, _)| *id == native)?;
            Some(match entry {
                ApiEntry::Function(f) => f.doc.as_str(),
                ApiEntry::Method(m) => m.doc.as_str(),
                ApiEntry::Constant(c) => c.doc.as_str(),
            })
        };
        // host items are declared in rust, so their docs come from the library instead
        let host_docs = match dec_id.map(|dec| (dec, &resolutions.decs[dec].kind)) {
            Some((
                _,
                ResolvedDeclKind::Item {
                    native: Some(native),
                    ..
                },
            )) => native_doc(*native),
            Some((dec, ResolvedDeclKind::Constant(_))) => resolutions
                .native_constants
                .get(&dec)
                .and_then(|native| native_doc(*native)),
            Some((_, ResolvedDeclKind::Adt(aid))) => adt(*aid).map(|adt| adt.doc.as_str()),
            Some((_, ResolvedDeclKind::Variant { parent, layout })) => adt(*parent)
                .and_then(|adt| adt.variants.iter().find(|v| v.layout_id == *layout))
                .map(|variant| variant.doc.as_str()),
            None => match ty {
                Ty::Adt(aid) => adt(*aid).map(|adt| adt.doc.as_str()),
                _ => None,
            },
            _ => None,
        };
        let docs = dec_id
            .and_then(|dec| {
                let declared = resolutions.decs[dec].location;
                let (_, declaring) = self.files.get_index(declared.file_id)?;
                declaring.ast.docs_for(declared.span.start)
            })
            .or(host_docs.filter(|doc| !doc.is_empty()));
        let value = match docs {
            Some(docs) => format!("```mimas\n{text}\n```\n\n---\n\n{docs}"),
            None => format!("```mimas\n{text}\n```"),
        };

        Some(Hover {
            contents: Contents::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: file.range(span),
        })
    }

    /// Where the name at `position` was declared.
    pub fn definition(&self, path: &Path, position: Position) -> Option<Location> {
        let (resolutions, dec) = self.dec_at(path, position)?;
        let dec = &resolutions.decs[dec];
        let span = dec.location.span;
        // natives, builtins, and module segments are declared with no source behind them
        if span.is_empty() {
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

    /// The range F2 should offer to edit, or nothing when the name can't be renamed.
    pub fn prepare_rename(&self, path: &Path, position: Position) -> Option<Range> {
        let (resolutions, target) = self.dec_at(path, position)?;
        self.family(resolutions, target).ok()?;
        let file = self.files.get(path)?;
        let (_, span) = file.ast.node_at(file.offset(position)?)?;
        file.range(span)
    }

    /// Renames the name at `position` everywhere it appears, or says why it can't.
    pub fn rename(
        &self,
        path: &Path,
        position: Position,
        new_name: &str,
    ) -> Result<WorkspaceEdit, String> {
        if !is_identifier(new_name) {
            return Err(format!("`{new_name}` isn't a valid mimas name"));
        }
        if self.resolutions.is_err() {
            return Err("renaming needs the project to check cleanly first".to_owned());
        }
        let (resolutions, target) = self
            .dec_at(path, position)
            .ok_or("there is nothing to rename here")?;
        let family = self.family(resolutions, target)?;

        // every written occurrence of anything in the family, per file
        let mut spans: Vec<Vec<Span>> = self.files.iter().map(|_| Vec::new()).collect();
        for (file_id, (_, file)) in self.files.iter().enumerate() {
            for (id, span) in file.idents() {
                if resolutions
                    .node_decs
                    .get(&id)
                    .is_some_and(|dec| family.contains(dec))
                {
                    spans[file_id].push(span);
                }
            }
        }

        // the scopes are gone by now, so the check for a name that captures something else is to
        // make the edit and see whether the project still solves
        let probe = self
            .files
            .iter()
            .zip(&spans)
            .map(|((path, file), spans)| (path.clone(), renamed(&file.text, spans, new_name)))
            .collect();
        let probe = Analysis::load(probe, self.library.clone());
        let Ok(probed) = probe.resolutions.as_ref() else {
            return Err(format!("renaming to `{new_name}` would not compile"));
        };
        if shape(&self.files, resolutions) != shape(&probe.files, probed) {
            return Err(format!(
                "renaming to `{new_name}` would change what another name refers to"
            ));
        }

        let changes = self
            .files
            .iter()
            .zip(&spans)
            .filter(|(_, spans)| !spans.is_empty())
            .filter_map(|((path, file), spans)| {
                let edits = spans
                    .iter()
                    .filter_map(|span| {
                        Some(TextEdit {
                            range: file.range(*span)?,
                            new_text: new_name.to_owned(),
                        })
                    })
                    .collect();
                Some((Uri::from_file_path(path).ok()?, edits))
            })
            .collect();

        return Ok(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        });

        /// Whether the lexer reads this as one plain name.
        fn is_identifier(name: &str) -> bool {
            let mut lexer = Lexer::new(name, 0, "<rename>".to_owned());
            let first = matches!(
                lexer.next().map(|tok| tok.kind),
                Some(TokKind::Ident(lexeme)) if lexeme == name
            );
            first && lexer.next().is_none()
        }

        /// How a project's idents group by what they resolve to, in walk order. Two of these
        /// differ when an edit quietly rebinds a name it didn't touch.
        fn shape(
            files: &IndexMap<PathBuf, SourceFile>,
            resolutions: &Resolutions,
        ) -> Vec<Vec<usize>> {
            files
                .values()
                .map(|file| {
                    let mut seen: HashMap<DecId, usize> = HashMap::new();
                    file.idents()
                        .into_iter()
                        .map(|(id, _)| match resolutions.node_decs.get(&id) {
                            Some(dec) => {
                                let next = seen.len() + 1;
                                *seen.entry(*dec).or_insert(next)
                            }
                            None => 0,
                        })
                        .collect()
                })
                .collect()
        }

        /// `text` with every span replaced. Back to front, so the earlier spans keep their offsets.
        fn renamed(text: &str, spans: &[Span], new_name: &str) -> String {
            let mut spans = spans.to_vec();
            spans.sort_by_key(|span| std::cmp::Reverse(span.start));
            let mut text = text.to_owned();
            for span in spans {
                text.replace_range(span.start..span.end, new_name);
            }
            text
        }
    }

    /// Every dec that has to move with `target`. A pact member brings its whole family: the
    /// pact's own declaration and the matching method on each implementor.
    fn family(&self, resolutions: &Resolutions, target: DecId) -> Result<Vec<DecId>, String> {
        let declared = |dec: &DecId| !resolutions.decs[*dec].location.span.is_empty();
        if !declared(&target) {
            return Err("that name is built in, so there is no mimas source to rename".to_owned());
        }

        let head = resolutions.decs[target].implements.unwrap_or(target);
        let family: Vec<DecId> = resolutions
            .decs
            .iter()
            .filter(|(dec_id, dec)| *dec_id == head || dec.implements == Some(head))
            .map(|(dec_id, _)| dec_id)
            .collect();
        if !family.iter().all(declared) {
            return Err("part of this pact has no mimas source to rename".to_owned());
        }
        Ok(family)
    }

    /// The declaration the name at `position` resolves to.
    fn dec_at(&self, path: &Path, position: Position) -> Option<(&Resolutions, DecId)> {
        let resolutions = self.resolutions.as_ref().ok()?;
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

        let resolutions = self.resolutions.as_ref().ok()?;
        let file = self.files.get(path)?;
        let mut bindings = Bindings::default();
        walk_stmts(file.ast.stmts(), &mut bindings);

        let hints = bindings
            .0
            .into_iter()
            .filter_map(|(id, span)| {
                let dec = *resolutions.node_decs.get(&id)?;
                let position = file.range(span)?.end;
                if !(range.start..=range.end).contains(&position) {
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

    /// The errors that point into `path`.
    pub fn diagnostics(&self, path: &Path) -> Vec<Diagnostic> {
        let (Err(errors), Some(file_id)) = (&self.resolutions, self.files.get_index_of(path))
        else {
            return Vec::new();
        };
        errors
            .iter()
            .filter(|error| self.file_of(error) == Some(file_id))
            .map(|error| self.files[file_id].diagnostic(error))
            .collect()
    }

    /// Which file an error points into, by the name its source was registered under.
    fn file_of(&self, error: &shared::Error) -> Option<FileId> {
        let label = error.labels()?.next()?;
        let contents = error.source_code()?.read_span(label.inner(), 0, 0).ok()?;
        self.files.get_index_of(Path::new(contents.name()?))
    }
}
