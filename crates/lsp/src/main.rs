mod analysis;
mod host_api;
mod project;
mod server;
mod source_file;
mod workspace;

use std::path::PathBuf;

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

    // the host API manifest, found from the first workspace folder
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
    // `expect_api` is whether a missing manifest is worth telling the user about
    let (api_path, expect_api) = match params["initializationOptions"]["apiPath"].as_str() {
        Some("off") => (None, false),
        Some(path) if !path.is_empty() => (Some(root.join(path)), true),
        _ => {
            // cargo knows the real target dir: a parent workspace, CARGO_TARGET_DIR, or a
            // configured target-dir. it fails outside a cargo project
            let target_dir = std::process::Command::new("cargo")
                .args(["metadata", "--format-version", "1", "--no-deps"])
                .current_dir(&root)
                .output()
                .ok()
                .filter(|out| out.status.success())
                .and_then(|out| serde_json::from_slice::<serde_json::Value>(&out.stdout).ok())
                .and_then(|meta| meta["target_directory"].as_str().map(PathBuf::from));
            let expect_api = target_dir.is_some();
            let target_dir = target_dir.unwrap_or_else(|| root.join("target"));
            (Some(target_dir.join("mimas").join("api.json")), expect_api)
        }
    };

    Server::new(&connection, api_path, expect_api)?.run()?;
    // the writer thread only stops once its sender is gone
    drop(connection);
    io_threads.join()?;
    Ok(())
}
