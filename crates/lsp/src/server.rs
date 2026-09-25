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
                Err(reason) => Response::new_err(req.id, ErrorCode::RequestFailed as i32, reason),
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
        let (diagnostics, messages) = self.workspace.update(&path, text);
        for (uri, diagnostics) in diagnostics {
            self.publish_diagnostics(uri, diagnostics)?;
        }
        for (kind, message) in messages {
            self.show_message(kind, message)?;
        }
        Ok(())
    }

    fn hover(&self, params: HoverParams) -> Option<Hover> {
        let position = params.text_document_position_params;
        let path = position.text_document.uri.to_file_path().ok()?;
        let project = self.workspace.project(&path)?;
        project
            .analysis(&path)?
            .hover(&path, position.position, &project.library)
    }

    fn definition(&self, params: DefinitionParams) -> Option<Location> {
        let position = params.text_document_position_params;
        let path = position.text_document.uri.to_file_path().ok()?;
        self.workspace
            .analysis(&path)?
            .definition(&path, position.position)
    }

    fn prepare_rename(&self, params: PrepareRenameParams) -> Option<Range> {
        let position = params.text_document_position_params;
        let path = position.text_document.uri.to_file_path().ok()?;
        self.workspace
            .analysis(&path)?
            .prepare_rename(&path, position.position)
    }

    fn rename(&self, params: RenameParams) -> Result<WorkspaceEdit, String> {
        let position = params.text_document_position_params;
        let path = position
            .text_document
            .uri
            .to_file_path()
            .map_err(|_| "that file isn't on disk".to_owned())?;
        let project = self
            .workspace
            .project(&path)
            .ok_or("that file isn't part of a project")?;
        project.rename(&path, position.position, &params.new_name)
    }

    fn inlay_hints(&self, params: InlayHintParams) -> Option<Vec<InlayHint>> {
        let path = params.text_document.uri.to_file_path().ok()?;
        self.workspace
            .analysis(&path)?
            .inlay_hints(&path, params.range)
    }

    fn highlights(&self, params: DocumentHighlightParams) -> Option<Vec<DocumentHighlight>> {
        let position = params.text_document_position_params;
        let path = position.text_document.uri.to_file_path().ok()?;
        self.workspace
            .analysis(&path)?
            .highlights(&path, position.position)
    }

    fn references(&self, params: ReferenceParams) -> Option<Vec<Location>> {
        let position = params.text_document_position_params;
        let path = position.text_document.uri.to_file_path().ok()?;
        let with_declaration = params.context.include_declaration;
        self.workspace
            .project(&path)?
            .references(&path, position.position, with_declaration)
    }

    fn symbols(&self, params: DocumentSymbolParams) -> Option<Vec<DocumentSymbol>> {
        let path = params.text_document.uri.to_file_path().ok()?;
        self.workspace.analysis(&path)?.symbols(&path)
    }

    fn show_message(&self, kind: MessageType, message: String) -> anyhow::Result<()> {
        let note = Notification::new(
            LspNotificationMethod::WindowShowMessage.as_str().to_owned(),
            ShowMessageParams { kind, message },
        );
        self.connection.sender.send(Message::Notification(note))?;
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
