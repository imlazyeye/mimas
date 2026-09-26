mod analysis;
mod host_api;
mod server;
mod solved;
mod source_file;
mod workspace;

use host_api::HostApi;
use lsp_server::Connection;
use lsp_types::*;
use server::Server;

fn main() -> anyhow::Result<()> {
    let (connection, io_threads) = Connection::stdio();
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSync::Kind(TextDocumentSyncKind::Full)),
        hover_provider: Some(HoverProvider::Bool(true)),
        definition_provider: Some(DefinitionProvider::Bool(true)),
        document_symbol_provider: Some(DocumentSymbolProvider::Bool(true)),
        references_provider: Some(ReferencesProvider::Bool(true)),
        document_highlight_provider: Some(DocumentHighlightProvider::Bool(true)),
        inlay_hint_provider: Some(InlayHintProvider::Bool(true)),
        rename_provider: Some(RenameProvider::RenameOptions(RenameOptions {
            prepare_provider: Some(true),
            ..Default::default()
        })),
        ..Default::default()
    };
    let params = connection.initialize(serde_json::to_value(capabilities)?)?;

    // `apiPath` swaps each project's own host API for one manifest or std alone. A relative path
    // starts where the editor started the server (in VS Code, the first workspace folder).
    let host_api = match params["initializationOptions"]["apiPath"].as_str() {
        Some("off") => HostApi::Off,
        Some(path) if !path.is_empty() => HostApi::Manifest(path.into()),
        _ => HostApi::Packages,
    };

    Server::new(&connection, host_api).run()?;
    // the writer thread only stops once its sender is gone
    drop(connection);
    io_threads.join()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    mod host_api;
    mod solved;
    mod utils;
    mod workspace;
}
