use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use api::Library;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::{
    DefinitionParams, DefinitionRequest, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentHighlight, DocumentHighlightParams,
    DocumentHighlightRequest, DocumentSymbol, DocumentSymbolParams, DocumentSymbolRequest, Hover,
    HoverParams, HoverRequest, InlayHint, InlayHintParams, InlayHintRequest, Location,
    LspNotificationMethod, PrepareRenameParams, PrepareRenameRequest, PublishDiagnosticsParams,
    Range, ReferenceParams, ReferencesRequest, RenameParams, RenameRequest,
    TextDocumentContentChangeEvent, Uri, WorkspaceEdit,
};
use serde::{Serialize, de::DeserializeOwned};

use crate::project::Project;

/// The protocol's code for "understood, but couldn't be done" -- clients surface the message.
const REQUEST_FAILED: i32 = -32803;

pub struct Server<'a> {
    connection: &'a Connection,
    library: Library<()>,
    /// The editor's text for every open file, which wins over what's on disk.
    open: HashMap<PathBuf, String>,
    /// Keyed by root (the directory holding `main.mim`, or the lone file itself).
    projects: HashMap<PathBuf, Project>,
}

impl<'a> Server<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        let library = vm::Vm::new().install_library(library::std);
        Self {
            connection,
            library,
            open: HashMap::new(),
            projects: HashMap::new(),
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        for message in &self.connection.receiver {
            match message {
                Message::Request(req) => {
                    if self.connection.handle_shutdown(&req)? {
                        return Ok(());
                    }
                    self.handle_request(req)?;
                }
                Message::Notification(note) => self.handle_notification(note)?,
                Message::Response(_) => {}
            }
        }
        Ok(())
    }

    fn handle_request(&self, req: Request) -> anyhow::Result<()> {
        let mut req = Some(req);
        self.on::<HoverRequest, _>(&mut req, Self::hover)?;
        self.on::<DefinitionRequest, _>(&mut req, Self::definition)?;
        self.on::<DocumentSymbolRequest, _>(&mut req, Self::symbols)?;
        self.on::<ReferencesRequest, _>(&mut req, Self::references)?;
        self.on::<DocumentHighlightRequest, _>(&mut req, Self::highlights)?;
        self.on::<InlayHintRequest, _>(&mut req, Self::inlay_hints)?;
        self.on::<PrepareRenameRequest, _>(&mut req, Self::prepare_rename)?;
        self.on_fallible::<RenameRequest, _>(&mut req, Self::rename)?;

        // anything still here is a method we never advertised
        if let Some(req) = req {
            let unhandled = Response::new_err(
                req.id,
                ErrorCode::MethodNotFound as i32,
                format!("unhandled method {}", req.method),
            );
            self.connection.sender.send(Message::Response(unhandled))?;
        }
        Ok(())
    }

    /// Answers `req` if it is an `R`, and leaves it alone otherwise.
    fn on<R: lsp_types::Request, T: Serialize>(
        &self,
        req: &mut Option<Request>,
        handle: impl FnOnce(&Self, R::Params) -> T,
    ) -> anyhow::Result<()> {
        self.on_fallible::<R, T>(req, |server, params| Ok(handle(server, params)))
    }

    /// [`Self::on`] for a request that can refuse, with a reason the client shows. Params that
    /// don't parse are refused the same way rather than taking the server down with them.
    fn on_fallible<R: lsp_types::Request, T: Serialize>(
        &self,
        req: &mut Option<Request>,
        handle: impl FnOnce(&Self, R::Params) -> Result<T, String>,
    ) -> anyhow::Result<()> {
        let Some(req) = req.take_if(|req| req.method == R::METHOD.as_str()) else {
            return Ok(());
        };
        let response = match serde_json::from_value::<R::Params>(req.params) {
            Ok(params) => match handle(self, params) {
                Ok(result) => Response::new_ok(req.id, result),
                Err(reason) => Response::new_err(req.id, REQUEST_FAILED, reason),
            },
            Err(e) => Response::new_err(req.id, ErrorCode::InvalidParams as i32, e.to_string()),
        };
        self.connection.sender.send(Message::Response(response))?;
        Ok(())
    }

    fn handle_notification(&mut self, note: Notification) -> anyhow::Result<()> {
        // every document notification comes down to a uri and its new text (none once closed)
        let (uri, text) = match LspNotificationMethod::from(note.method.as_str()) {
            LspNotificationMethod::TextDocumentDidOpen => {
                let Some(p) = params::<DidOpenTextDocumentParams>(note.params) else {
                    return Ok(());
                };
                (p.text_document.uri, Some(p.text_document.text))
            }
            LspNotificationMethod::TextDocumentDidChange => {
                let Some(p) = params::<DidChangeTextDocumentParams>(note.params) else {
                    return Ok(());
                };
                // full sync, so the last whole-document change is the text
                let text = p
                    .content_changes
                    .into_iter()
                    .rev()
                    .find_map(|change| match change {
                        TextDocumentContentChangeEvent::TextDocumentContentChangeWholeDocument(
                            c,
                        ) => Some(c.text),
                        _ => None,
                    });
                let Some(text) = text else { return Ok(()) };
                (p.text_document.text_document_identifier.uri, Some(text))
            }
            LspNotificationMethod::TextDocumentDidClose => {
                let Some(p) = params::<DidCloseTextDocumentParams>(note.params) else {
                    return Ok(());
                };
                (p.text_document.uri, None)
            }
            _ => return Ok(()),
        };
        let Ok(path) = uri.to_file_path() else {
            return Ok(());
        };
        match text {
            Some(text) => self.open.insert(path.clone(), text),
            None => self.open.remove(&path),
        };
        self.reanalyze(&path)
    }

    fn hover(&self, params: HoverParams) -> Option<Hover> {
        let position = params.text_document_position_params;
        let path = position.text_document.uri.to_file_path().ok()?;
        self.projects
            .values()
            .find_map(|project| project.hover(&path, position.position))
    }

    fn definition(&self, params: DefinitionParams) -> Option<Location> {
        let position = params.text_document_position_params;
        let path = position.text_document.uri.to_file_path().ok()?;
        self.projects
            .values()
            .find_map(|project| project.definition(&path, position.position))
    }

    fn prepare_rename(&self, params: PrepareRenameParams) -> Option<Range> {
        let position = params.text_document_position_params;
        let path = position.text_document.uri.to_file_path().ok()?;
        self.projects
            .values()
            .find_map(|project| project.prepare_rename(&path, position.position))
    }

    fn rename(&self, params: RenameParams) -> Result<WorkspaceEdit, String> {
        let position = params.text_document_position_params;
        let path = position
            .text_document
            .uri
            .to_file_path()
            .map_err(|_| "that file isn't on disk".to_owned())?;
        let project = self
            .projects
            .values()
            .find(|project| project.files.contains_key(&path))
            .ok_or("that file isn't part of a project")?;
        project.rename(&path, position.position, &params.new_name, &self.library)
    }

    fn inlay_hints(&self, params: InlayHintParams) -> Option<Vec<InlayHint>> {
        let path = params.text_document.uri.to_file_path().ok()?;
        self.projects
            .values()
            .find_map(|project| project.inlay_hints(&path, params.range))
    }

    fn highlights(&self, params: DocumentHighlightParams) -> Option<Vec<DocumentHighlight>> {
        let position = params.text_document_position_params;
        let path = position.text_document.uri.to_file_path().ok()?;
        self.projects
            .values()
            .find_map(|project| project.highlights(&path, position.position))
    }

    fn references(&self, params: ReferenceParams) -> Option<Vec<Location>> {
        let position = params.text_document_position_params;
        let path = position.text_document.uri.to_file_path().ok()?;
        let with_declaration = params.context.include_declaration;
        self.projects
            .values()
            .find_map(|project| project.references(&path, position.position, with_declaration))
    }

    fn symbols(&self, params: DocumentSymbolParams) -> Option<Vec<DocumentSymbol>> {
        let path = params.text_document.uri.to_file_path().ok()?;
        self.projects
            .values()
            .find_map(|project| project.symbols(&path))
    }

    /// Reloads the project `path` belongs to, dropping any project it used to belong to, and
    /// publishes diagnostics for every file in it (and clears them for files that left).
    fn reanalyze(&mut self, path: &Path) -> anyhow::Result<()> {
        let root = path
            .ancestors()
            .skip(1)
            .find(|dir| dir.join("main.mim").is_file())
            .map_or_else(|| path.to_path_buf(), Path::to_path_buf);
        let mut paths = if root == path {
            vec![path.to_path_buf()]
        } else {
            solve::mim_files(&root).0
        };
        // open files aren't necessarily on disk yet, but are still part of the project
        for open in self.open.keys() {
            let in_root = if root == path {
                open == path
            } else {
                open.starts_with(&root)
            };
            if in_root && !paths.contains(open) {
                paths.push(open.clone());
            }
        }
        paths.sort();

        let mut files = Vec::new();
        for path in paths {
            let text = match self.open.get(&path) {
                Some(text) => text.clone(),
                None => match std::fs::read_to_string(&path) {
                    Ok(text) => text,
                    Err(_) => continue,
                },
            };
            files.push((path, text));
        }

        // the projects this replaces: the one at this root, anything under it, and any other
        // that had this file (its root moved). their files start with cleared diagnostics
        let mut cleared: Vec<Uri> = Vec::new();
        self.projects.retain(|key, project| {
            let stale = key.starts_with(&root) || project.files.contains_key(path);
            if stale {
                cleared.extend(
                    project
                        .files
                        .keys()
                        .filter_map(|p| Uri::from_file_path(p).ok()),
                );
            }
            !stale
        });

        let project = Project::load(files, &self.library);
        for (uri, diagnostics) in project.diagnostics() {
            cleared.retain(|c| *c != uri);
            self.publish_diagnostics(uri, diagnostics)?;
        }
        for uri in cleared {
            self.publish_diagnostics(uri, Vec::new())?;
        }
        if !project.files.is_empty() {
            self.projects.insert(root, project);
        }
        Ok(())
    }

    fn publish_diagnostics(
        &self,
        uri: Uri,
        diagnostics: Vec<lsp_types::Diagnostic>,
    ) -> anyhow::Result<()> {
        let params = PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: None,
        };
        let note = Notification::new(
            LspNotificationMethod::TextDocumentPublishDiagnostics
                .as_str()
                .to_owned(),
            params,
        );
        self.connection.sender.send(Message::Notification(note))?;
        Ok(())
    }
}

/// A message with params we can't read is logged and skipped rather than taking the server down.
fn params<T: DeserializeOwned>(value: serde_json::Value) -> Option<T> {
    serde_json::from_value(value)
        .map_err(|e| eprintln!("mimas-lsp: malformed params: {e}"))
        .ok()
}
