use std::path::PathBuf;

use api::{Library, Manifest, Project};

/// Where projects get their host API from.
pub enum HostApi {
    /// The newest manifest written by the binaries of the cargo package a project sits in. A
    /// project outside a package gets std alone.
    Packages,
    /// One manifest for every project (`mimas.apiPath`).
    Manifest(PathBuf),
    /// std alone for every project.
    Off,
}

impl HostApi {
    /// The library to check `project` against, read fresh from its manifest. It's std alone when
    /// there's no manifest to go by, along with why when one was expected.
    pub fn library(&self, project: &Project) -> (Library<()>, Option<String>) {
        // a problem ends with how to write a manifest this server can read
        let (manifest, fix) = match self {
            HostApi::Packages => (
                Manifest::find(project),
                "Run the binary with `cargo run` to write it, or write it manually with \
                 `mimas::write_api` and point `mimas.apiPath` at it.",
            ),
            HostApi::Manifest(path) => (
                Manifest::read(path).map(Some),
                "Write it with `mimas::write_api`, or clear `mimas.apiPath`.",
            ),
            HostApi::Off => (Ok(None), ""),
        };
        let std_alone = || vm::Vm::new().install_library(library::std);
        match manifest {
            Ok(Some(manifest)) => (manifest.library, None),
            Ok(None) => (std_alone(), None),
            Err(problem) => {
                let message =
                    format!("{problem}, so scripts get the standard library alone. {fix}");
                (std_alone(), Some(message))
            }
        }
    }
}
