use serde::{Deserialize, Serialize};

use crate::Library;

/// The version a manifest was written with. Builtin ADT ids and std's layout move between
/// versions, so a manifest from another version is not safe to load.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A host's installed API, as written to `api.json` and read back by the language server.
#[derive(Deserialize)]
pub struct Manifest {
    pub version: String,
    pub library: Library<()>,
}

/// The writing side of [Manifest]: the same shape, borrowing the host's library.
#[derive(Serialize)]
pub struct ManifestRef<'a> {
    pub version: &'a str,
    pub library: &'a Library<()>,
}
