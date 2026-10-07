//! The language server's analysis as a library too, so a host that edits mimas in its own UI can
//! answer hover, definition, rename and the rest from an [`Analysis`] without the protocol. The
//! `server` feature adds the server itself.
pub mod analysis;
#[cfg(feature = "server")]
pub mod host_api;
#[cfg(feature = "server")]
pub mod server;
pub mod solved;
pub mod source_file;
#[cfg(feature = "server")]
pub mod workspace;

pub use analysis::Analysis;
pub use lsp_types;
