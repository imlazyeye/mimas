use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    rc::Rc,
    time::SystemTime,
};

use api::Library;
use lsp_types::MessageType;

/// The libraries projects are checked against, each read from a manifest a host wrote.
pub struct HostApis {
    source: Source,
    std: Rc<Library<()>>,
    /// The manifests the binaries of each cargo package write.
    binaries: HashMap<PathBuf, Vec<PathBuf>>,
    /// Each manifest read so far, with when it was written.
    loaded: HashMap<PathBuf, (SystemTime, Rc<Library<()>>)>,
    /// Packages (or the configured manifest) the user already heard are missing one.
    missing: HashSet<PathBuf>,
    /// What the user should hear about.
    pub messages: Vec<(MessageType, String)>,
}

/// Where projects get their host API from.
pub enum Source {
    /// The newest manifest written by the binaries of the cargo package a project sits in. A
    /// project outside a package gets std alone.
    Packages,
    /// One manifest for every project (`mimas.apiPath`).
    Manifest(PathBuf),
    /// std alone for every project.
    Off,
}

impl HostApis {
    pub fn new(source: Source) -> Self {
        Self {
            source,
            std: Rc::new(vm::Vm::new().install_library(library::std)),
            binaries: HashMap::new(),
            loaded: HashMap::new(),
            missing: HashSet::new(),
            messages: Vec::new(),
        }
    }

    /// The library to check a project against, given the cargo package it sits in. It's the same
    /// `Rc` until the manifest it comes from changes.
    pub fn library(&mut self, package: Option<&Path>) -> Rc<Library<()>> {
        let (key, manifests) = match (&self.source, package) {
            (Source::Packages, Some(package)) => (
                package,
                self.binaries
                    .entry(package.to_path_buf())
                    .or_insert_with(|| binaries(package))
                    .as_slice(),
            ),
            (Source::Manifest(path), _) => (path.as_path(), std::slice::from_ref(path)),
            _ => return self.std.clone(),
        };
        let newest = manifests
            .iter()
            .filter_map(|path| Some((modified(path)?, path.clone())))
            .max();
        let Some((modified, path)) = newest else {
            if !manifests.is_empty() && self.missing.insert(key.to_path_buf()) {
                let message = format!(
                    "mimas: no host API for {} yet, run your Rust host once to write it",
                    key.display()
                );
                self.messages.push((MessageType::Info, message));
            }
            return self.std.clone();
        };
        return self.load(&path, modified);

        // every bin and example target of the package, each writing its own manifest
        fn binaries(package: &Path) -> Vec<PathBuf> {
            // cargo knows the real target dir: a parent workspace, CARGO_TARGET_DIR, or a
            // configured target-dir
            let Some(meta) = std::process::Command::new("cargo")
                .args(["metadata", "--format-version", "1", "--no-deps"])
                .current_dir(package)
                .output()
                .ok()
                .filter(|out| out.status.success())
                .and_then(|out| serde_json::from_slice::<serde_json::Value>(&out.stdout).ok())
            else {
                return Vec::new();
            };
            let dir = PathBuf::from(meta["target_directory"].as_str().unwrap_or_default());
            let manifest = std::fs::canonicalize(package.join("Cargo.toml")).ok();
            meta["packages"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|package| package["manifest_path"].as_str().map(PathBuf::from) == manifest)
                .flat_map(|package| package["targets"].as_array().into_iter().flatten())
                .filter(|target| {
                    target["kind"].as_array().is_some_and(|kinds| {
                        kinds.iter().any(|kind| kind == "bin" || kind == "example")
                    })
                })
                .filter_map(|target| Some(api::manifest_file(&dir, target["name"].as_str()?)))
                .collect()
        }
    }

    /// The library in the manifest at `path`, last written at `modified`, read again only once
    /// that changes. It's std alone when the manifest is unusable, and the user hears why.
    fn load(&mut self, path: &Path, modified: SystemTime) -> Rc<Library<()>> {
        if let Some((when, library)) = self.loaded.get(path)
            && *when == modified
        {
            return library.clone();
        }

        // the host may be halfway through writing it, so leave it for the next look
        let Some(json) = std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        else {
            return match self.loaded.get(path) {
                Some((_, library)) => library.clone(),
                None => self.std.clone(),
            };
        };
        let version = json["version"].as_str().unwrap_or("unknown").to_owned();
        let manifest = if version != api::VERSION {
            Err(format!(
                "{} is from mimas {version}, rebuild the host against mimas {}",
                path.display(),
                api::VERSION
            ))
        } else {
            serde_json::from_value::<api::Manifest>(json)
                .map_err(|e| format!("couldn't read {}: {e}", path.display()))
        };
        let library = match manifest {
            Ok(manifest) => Rc::new(manifest.library),
            Err(problem) => {
                eprintln!("mimas-lsp: {problem}");
                self.messages
                    .push((MessageType::Warning, format!("mimas: {problem}")));
                self.std.clone()
            }
        };
        self.loaded
            .insert(path.to_path_buf(), (modified, library.clone()));
        library
    }
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}
