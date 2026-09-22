use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use api::Library;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::{
    DefinitionParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentSymbol, DocumentSymbolParams, Hover, HoverParams, Location,
    LspNotificationMethod, LspRequestMethod, PublishDiagnosticsParams,
    TextDocumentContentChangeEvent, Uri,
};
use serde::de::DeserializeOwned;

use crate::project::Project;

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

    fn handle_request(&mut self, req: Request) -> anyhow::Result<()> {
        let response = match LspRequestMethod::from(req.method.as_str()) {
            LspRequestMethod::TextDocumentHover => match params::<HoverParams>(req.params) {
                Some(params) => Response::new_ok(req.id, self.hover(params)),
                None => Response::new_err(
                    req.id,
                    ErrorCode::InvalidParams as i32,
                    "malformed hover params".to_owned(),
                ),
            },
            LspRequestMethod::TextDocumentDefinition => {
                match params::<DefinitionParams>(req.params) {
                    Some(params) => Response::new_ok(req.id, self.definition(params)),
                    None => Response::new_err(
                        req.id,
                        ErrorCode::InvalidParams as i32,
                        "malformed definition params".to_owned(),
                    ),
                }
            }
            LspRequestMethod::TextDocumentDocumentSymbol => {
                match params::<DocumentSymbolParams>(req.params) {
                    Some(params) => Response::new_ok(req.id, self.symbols(params)),
                    None => Response::new_err(
                        req.id,
                        ErrorCode::InvalidParams as i32,
                        "malformed document symbol params".to_owned(),
                    ),
                }
            }
            _ => Response::new_err(
                req.id,
                ErrorCode::MethodNotFound as i32,
                format!("unhandled method {}", req.method),
            ),
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
