mod project;
mod server;
mod source_file;

use lsp_server::Connection;
use lsp_types::{
    DefinitionProvider, DocumentSymbolProvider, HoverProvider, ServerCapabilities,
    TextDocumentSync, TextDocumentSyncKind,
};
use server::Server;

fn main() -> anyhow::Result<()> {
    let (connection, io_threads) = Connection::stdio();
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSync::Kind(TextDocumentSyncKind::Full)),
        hover_provider: Some(HoverProvider::Bool(true)),
        definition_provider: Some(DefinitionProvider::Bool(true)),
        document_symbol_provider: Some(DocumentSymbolProvider::Bool(true)),
        ..Default::default()
    };
    connection.initialize(serde_json::to_value(capabilities)?)?;
    Server::new(&connection).run()?;
    // the writer thread only stops once its sender is gone
    drop(connection);
    io_threads.join()?;
    Ok(())
}
