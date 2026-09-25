mod analysis;
mod host_api;
mod project;
mod server;
mod source_file;
mod workspace;

#[cfg(test)]
mod tests {
    mod host_api;
    mod utils;
    mod workspace;
}

use host_api::Source;
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

    // `apiPath` swaps each project's own host API for one manifest (relative to the first
    // workspace folder) or std alone
    let file_path = |uri: &serde_json::Value| {
        serde_json::from_value::<Uri>(uri.clone())
            .ok()?
            .to_file_path()
            .ok()
    };
    let root = file_path(&params["workspaceFolders"][0]["uri"])
        .or_else(|| file_path(&params["rootUri"]))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default();
    let source = match params["initializationOptions"]["apiPath"].as_str() {
        Some("off") => Source::Off,
        Some(path) if !path.is_empty() => Source::Manifest(root.join(path)),
        _ => Source::Packages,
    };

    Server::new(&connection, source).run()?;
    // the writer thread only stops once its sender is gone
    drop(connection);
    io_threads.join()?;
    Ok(())
}
