use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use shared::Literal;

use crate::{ApiEntry, Library, Project, project::is_target_dir};

/// The version a manifest was written with. Builtin ADT ids and std's layout move between
/// versions, so a manifest from another version is not safe to load.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A host's installed API, as written to a manifest file and read back by the language server and
/// the cli.
#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub library: Library<()>,
}

impl Manifest {
    pub fn new(library: Library<()>) -> Self {
        Self {
            version: VERSION.to_owned(),
            library,
        }
    }

    /// The library scripts get from a lookup.
    pub fn library(
        found: Result<Option<Self>, String>,
        std: impl FnOnce() -> Library<()>,
    ) -> (Library<()>, Option<String>) {
        match found {
            Ok(Some(manifest)) => (manifest.library, None),
            Ok(None) => (std(), None),
            Err(problem) => (std(), Some(problem)),
        }
    }

    /// The newest manifest written by a binary of `project`'s cargo package. `None` outside a
    /// package, and for a package with no binaries (it hosts nothing, so there's no manifest to
    /// wait for).
    pub fn find(project: &Project) -> Result<Option<Self>, String> {
        // every bin and example target of the package, each writing its own manifest
        fn manifests(package: &Path) -> Vec<PathBuf> {
            // cargo knows the real target dir: a parent workspace, CARGO_TARGET_DIR, or a
            // configured target-dir
            let Some(meta) = std::process::Command::new("cargo")
                .args(["metadata", "--format-version", "1", "--no-deps"])
                .current_dir(package)
                .output()
                .ok()
                .and_then(|out| serde_json::from_slice::<serde_json::Value>(&out.stdout).ok())
            else {
                return Vec::new();
            };
            let dir = PathBuf::from(meta["target_directory"].as_str().unwrap_or_default());
            let Ok(manifest) = std::fs::canonicalize(package.join("Cargo.toml")) else {
                return Vec::new();
            };
            meta["packages"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|package| {
                    let path = package["manifest_path"].as_str().unwrap_or_default();
                    std::fs::canonicalize(path).is_ok_and(|path| path == manifest)
                })
                .flat_map(|package| package["targets"].as_array().into_iter().flatten())
                .filter(|target| matches!(target["kind"][0].as_str(), Some("bin" | "example")))
                .filter_map(|target| Some(Manifest::path(&dir, target["name"].as_str()?)))
                .collect()
        }

        let Some(package) = &project.package else {
            return Ok(None);
        };
        let paths = manifests(package);
        if paths.is_empty() {
            return Ok(None);
        }
        let newest = paths
            .iter()
            .filter_map(|path| Some((std::fs::metadata(path).ok()?.modified().ok()?, path)))
            .max();
        match newest {
            Some((_, path)) => Self::read(path).map(Some),
            None => Err(format!("no host API for {} yet", package.display())),
        }
    }

    /// Reads the manifest at `path`, as long as this version of mimas wrote it.
    pub fn read(path: &Path) -> Result<Self, String> {
        let bytes =
            std::fs::read(path).map_err(|e| format!("couldn't read {}: {e}", path.display()))?;
        let unreadable = |detail: serde_json::Error| {
            format!(
                "{} isn't a host API mimas can read ({detail})",
                path.display()
            )
        };
        let json: serde_json::Value = serde_json::from_slice(&bytes).map_err(unreadable)?;
        let version = json["version"].as_str().unwrap_or("unknown");
        if version != VERSION {
            return Err(format!(
                "{} is from mimas {version}, not mimas {VERSION}",
                path.display()
            ));
        }
        serde_json::from_value(json).map_err(unreadable)
    }

    /// Writes the manifest to `path` unless its output would be identical to what is already
    /// present.
    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        // serde_json writes these as `null`, which can't be read back
        for (_, entry) in self.library.natives() {
            if let ApiEntry::Constant(constant) = entry
                && let Literal::Float(value) = constant.value
                && !value.is_finite()
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("constant `{}` isn't a finite float", constant.name),
                ));
            }
        }

        let output = serde_json::to_vec_pretty(self).expect("failed to serialize the manifest!");

        // No need to write this if it's the same as last time
        if let Ok(data) = std::fs::read(path)
            && data == output
        {
            return Ok(());
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, output)
    }

    /// Where a host built as `exe` writes its manifest: [`Manifest::path`] in the cargo target dir
    /// it was built into. `None` for test and bench binaries and for anything outside a target dir.
    pub fn host_path(exe: &Path) -> Option<PathBuf> {
        let dir = exe.parent()?;
        if dir.file_name()? == "deps" {
            return None;
        }
        let target = dir.ancestors().find(|dir| is_target_dir(dir))?;
        Some(Self::path(target, &exe.file_stem()?.to_string_lossy()))
    }

    /// The manifest the binary `name` writes in the cargo target dir `target`
    /// (`<target>/mimas/<name>.json`). Each binary gets its own, so hosts sharing a target dir
    /// don't overwrite each other.
    pub fn path(target: &Path, name: &str) -> PathBuf {
        target.join("mimas").join(format!("{name}.json"))
    }
}
