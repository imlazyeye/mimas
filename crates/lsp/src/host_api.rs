use std::path::{Path, PathBuf};

use api::Library;

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
    /// The library to check a project in `package` against, read fresh from its manifest. It's std
    /// alone when there's no manifest to go by, along with why when one was expected.
    pub fn library(&self, package: Option<&Path>) -> (Library<()>, Option<String>) {
        let std_alone = || vm::Vm::new().install_library(library::std);
        let (owner, manifests) = match (self, package) {
            (HostApi::Packages, Some(package)) => (package, manifests(package)),
            (HostApi::Manifest(path), _) => (path.as_path(), vec![path.clone()]),
            _ => return (std_alone(), None),
        };
        // a package with no binaries hosts nothing, so it has no manifest to wait for
        if manifests.is_empty() {
            return (std_alone(), None);
        }
        let newest = manifests
            .iter()
            .filter_map(|path| Some((std::fs::metadata(path).ok()?.modified().ok()?, path)))
            .max();
        let problem = match newest {
            Some((_, path)) => match read(path) {
                Ok(library) => return (library, None),
                Err(problem) => problem,
            },
            None => format!(
                "no host API for {} yet, run your Rust host once to write it",
                owner.display()
            ),
        };
        return (std_alone(), Some(problem));

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
                // canonical on both sides, since on Windows only ours carries the `\\?\` prefix
                .filter(|package| {
                    let path = package["manifest_path"].as_str().unwrap_or_default();
                    std::fs::canonicalize(path).is_ok_and(|path| path == manifest)
                })
                .flat_map(|package| package["targets"].as_array().into_iter().flatten())
                .filter(|target| matches!(target["kind"][0].as_str(), Some("bin" | "example")))
                .filter_map(|target| Some(api::manifest_file(&dir, target["name"].as_str()?)))
                .collect()
        }

        fn read(path: &Path) -> Result<Library<()>, String> {
            let json = std::fs::read(path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
                .ok_or_else(|| format!("couldn't read {}", path.display()))?;
            let version = json["version"].as_str().unwrap_or("unknown");
            if version != api::VERSION {
                return Err(format!(
                    "{} is from mimas {version}, rebuild the host against mimas {}",
                    path.display(),
                    api::VERSION
                ));
            }
            serde_json::from_value::<api::Manifest>(json)
                .map(|manifest| manifest.library)
                .map_err(|e| format!("couldn't read {}: {e}", path.display()))
        }
    }
}
