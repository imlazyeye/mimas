use crate::{host_api::Source, workspace::Workspace};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::*;
use serde::{Serialize, de::DeserializeOwned};

pub struct Server<'a> {
    connection: &'a Connection,
    workspace: Workspace,
}

impl<'a> Server<'a> {
    pub fn new(connection: &'a Connection, source: Source) -> Self {
        Self {
            connection,
            workspace: Workspace::new(source),
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
        // most requests are about one file, answered by the analysis it belongs to
        let at = |params: TextDocumentPositionParams| {
            let path = params.text_document.uri.to_file_path().ok()?;
            Some((self.workspace.analysis(&path)?, path, params.position))
        };
        let file = |uri: Uri| {
            let path = uri.to_file_path().ok()?;
            Some((self.workspace.analysis(&path)?, path))
        };
        let mut req = Some(req);
        self.on::<HoverRequest, _>(&mut req, |params| {
            let (analysis, path, position) = at(params.text_document_position_params)?;
            analysis.hover(&path, position)
        })?;
        self.on::<DefinitionRequest, _>(&mut req, |params| {
            let (analysis, path, position) = at(params.text_document_position_params)?;
            analysis.definition(&path, position)
        })?;
        self.on::<DocumentSymbolRequest, _>(&mut req, |params| {
            let (analysis, path) = file(params.text_document.uri)?;
            analysis.symbols(&path)
        })?;
        self.on::<DocumentHighlightRequest, _>(&mut req, |params| {
            let (analysis, path, position) = at(params.text_document_position_params)?;
            analysis.highlights(&path, position)
        })?;
        self.on::<InlayHintRequest, _>(&mut req, |params| {
            let (analysis, path) = file(params.text_document.uri)?;
            analysis.inlay_hints(&path, params.range)
        })?;
        self.on::<PrepareRenameRequest, _>(&mut req, |params| {
            let (analysis, path, position) = at(params.text_document_position_params)?;
            analysis.prepare_rename(&path, position)
        })?;
        // references and rename reach past the file's own analysis, into the whole project
        self.on::<ReferencesRequest, _>(&mut req, |params| {
            let at = params.text_document_position_params;
            let path = at.text_document.uri.to_file_path().ok()?;
            let with_declaration = params.context.include_declaration;
            self.workspace
                .project(&path)?
                .references(&path, at.position, with_declaration)
        })?;
        self.on_fallible::<RenameRequest, _>(&mut req, |params| {
            let at = params.text_document_position_params;
            let refused = "that file isn't part of a project";
            let path = at.text_document.uri.to_file_path().map_err(|_| refused)?;
            let project = self.workspace.project(&path).ok_or(refused)?;
            project.rename(&path, at.position, &params.new_name)
        })?;

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
        handle: impl FnOnce(R::Params) -> T,
    ) -> anyhow::Result<()> {
        self.on_fallible::<R, T>(req, |params| Ok(handle(params)))
    }

    /// [`Self::on`] for a request that can refuse, with a reason the client shows. Params that
    /// don't parse are refused the same way rather than taking the server down with them.
    fn on_fallible<R: lsp_types::Request, T: Serialize>(
        &self,
        req: &mut Option<Request>,
        handle: impl FnOnce(R::Params) -> Result<T, String>,
    ) -> anyhow::Result<()> {
        let Some(req) = req.take_if(|req| req.method == R::METHOD.as_str()) else {
            return Ok(());
        };
        let response = match serde_json::from_value::<R::Params>(req.params) {
            Ok(params) => match handle(params) {
                Ok(result) => Response::new_ok(req.id, result),
                Err(reason) => Response::new_err(req.id, ErrorCode::RequestFailed as i32, reason),
            },
            Err(e) => Response::new_err(req.id, ErrorCode::InvalidParams as i32, e.to_string()),
        };
        self.connection.sender.send(Message::Response(response))?;
        Ok(())
    }

    fn handle_notification(&mut self, note: Notification) -> anyhow::Result<()> {
        let Some((path, text)) = document(note) else {
            return Ok(());
        };
        for (uri, diagnostics) in self.workspace.update(&path, text) {
            let params = PublishDiagnosticsParams {
                uri,
                diagnostics,
                version: None,
            };
            let method = LspNotificationMethod::TextDocumentPublishDiagnostics.as_str();
            let note = Notification::new(method.to_owned(), params);
            self.connection.sender.send(Message::Notification(note))?;
        }
        return Ok(());

        // every document notification comes down to a file and its new text (none once closed)
        fn document(note: Notification) -> Option<(std::path::PathBuf, Option<String>)> {
            use TextDocumentContentChangeEvent::TextDocumentContentChangeWholeDocument as Whole;
            let (uri, text) = match LspNotificationMethod::from(note.method.as_str()) {
                LspNotificationMethod::TextDocumentDidOpen => {
                    let p = params::<DidOpenTextDocumentParams>(note.params)?;
                    (p.text_document.uri, Some(p.text_document.text))
                }
                LspNotificationMethod::TextDocumentDidChange => {
                    let mut p = params::<DidChangeTextDocumentParams>(note.params)?;
                    // full sync, so the one change is the whole text
                    let Whole(change) = p.content_changes.pop()? else {
                        return None;
                    };
                    let uri = p.text_document.text_document_identifier.uri;
                    (uri, Some(change.text))
                }
                LspNotificationMethod::TextDocumentDidClose => {
                    let p = params::<DidCloseTextDocumentParams>(note.params)?;
                    (p.text_document.uri, None)
                }
                _ => return None,
            };
            Some((uri.to_file_path().ok()?, text))
        }

        /// A message with params we can't read is logged and skipped rather than taking the
        /// server down.
        fn params<T: DeserializeOwned>(value: serde_json::Value) -> Option<T> {
            serde_json::from_value(value)
                .map_err(|e| eprintln!("mimas-lsp: malformed params: {e}"))
                .ok()
        }
    }
}
