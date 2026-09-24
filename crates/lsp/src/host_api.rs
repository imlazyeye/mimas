use std::{path::PathBuf, time::SystemTime};

use api::Library;
use lsp_types::MessageType;

/// The host API manifest the server watches, loaded in place of std alone once a host has
/// written it.
pub struct HostApi {
    /// `None` when turned off.
    path: Option<PathBuf>,
    /// When the manifest the current library came from was written. `None` while it is std alone.
    modified: Option<SystemTime>,
    /// Tell the user once when the manifest is missing, since a host has to run to write it.
    notify_missing: bool,
}

impl HostApi {
    pub fn new(path: Option<PathBuf>, expect: bool) -> Self {
        Self {
            path,
            modified: None,
            notify_missing: expect,
        }
    }

    /// A library to swap in when the manifest changed since the last look, and anything the user
    /// should hear about.
    pub fn refresh(&mut self) -> (Option<Library<()>>, Option<(MessageType, String)>) {
        let Some(path) = &self.path else {
            return (None, None);
        };
        let modified = std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok();
        let missing = (modified.is_none() && std::mem::take(&mut self.notify_missing)).then(|| {
            let message = format!(
                "mimas: no host API at {} yet, run your Rust host once to write it",
                path.display()
            );
            (MessageType::Info, message)
        });
        if modified == self.modified {
            return (None, missing);
        }
        if modified.is_none() {
            self.modified = None;
            return (Some(std_library()), missing);
        }

        // the host may be halfway through writing it, so leave the time unset and retry next edit
        let Some(json) = std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        else {
            return (None, missing);
        };
        self.modified = modified;

        let version = json["version"].as_str().unwrap_or("unknown").to_owned();
        let problem = if version != api::VERSION {
            format!(
                "{} is from mimas {version}, rebuild the host against mimas {}",
                path.display(),
                api::VERSION
            )
        } else {
            match serde_json::from_value::<api::Manifest>(json) {
                Ok(manifest) => return (Some(manifest.library), missing),
                Err(e) => format!("couldn't read {}: {e}", path.display()),
            }
        };
        eprintln!("mimas-lsp: {problem}");
        let warning = (MessageType::Warning, format!("mimas: {problem}"));
        (Some(std_library()), Some(warning))
    }
}

pub fn std_library() -> Library<()> {
    vm::Vm::new().install_library(library::std)
}
