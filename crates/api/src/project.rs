use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// Where a file sits in a mimas project: the folder the project starts at, and the cargo package
/// that folder is in.
#[derive(Debug, PartialEq)]
pub struct Project {
    pub root: PathBuf,
    /// Whether the folders below `root` are part of the project.
    pub recursive: bool,
    pub package: Option<PathBuf>,
}

impl Project {
    /// The project of an absolute `path`. A folder is a project root as given, with everything
    /// below it. A file's project is the highest folder holding a `.mim` in the cargo package or
    /// repository the file is in, with everything below it, the way a host loads its whole scripts
    /// folder. Outside both, it's the file's own folder alone.
    pub fn of(path: &Path) -> Self {
        /// Whether `dir` directly holds a `.mim` file.
        fn holds_mim(dir: &Path) -> bool {
            std::fs::read_dir(dir).is_ok_and(|entries| {
                entries.flatten().any(|entry| {
                    entry.file_type().is_ok_and(|kind| kind.is_file()) && is_mim(&entry.path())
                })
            })
        }

        let file = path.is_file();
        let dir = match path.parent() {
            Some(dir) if file => dir,
            _ => path,
        };
        let mut root = dir;
        for ancestor in dir.ancestors() {
            if file && holds_mim(ancestor) {
                root = ancestor;
            }
            let package = is_package(ancestor);
            if package || ancestor.join(".git").exists() {
                return Self {
                    root: root.to_path_buf(),
                    recursive: true,
                    package: package.then(|| ancestor.to_path_buf()),
                };
            }
        }
        Self {
            root: dir.to_path_buf(),
            recursive: !file,
            package: None,
        }
    }

    /// Every `.mim` file in the project, sorted by path, with whatever the walk couldn't read.
    pub fn files(&self) -> (Vec<PathBuf>, Vec<std::io::Error>) {
        mim_files(&self.root, self.recursive)
    }
}

/// Every `.mim` file under `root` (or only directly in it, unless `recursive`), sorted by path,
/// with whatever the walk couldn't read. The walk leaves out any directory below `root` holding
/// another cargo package, and any cargo target dir.
fn mim_files(root: &Path, recursive: bool) -> (Vec<PathBuf>, Vec<std::io::Error>) {
    let mut files = Vec::new();
    let mut errors = Vec::new();
    let depth = if recursive { usize::MAX } else { 1 };
    let walk = WalkDir::new(root)
        .max_depth(depth)
        .into_iter()
        .filter_entry(|entry| {
            let dir = entry.path();
            entry.depth() == 0
                || !entry.file_type().is_dir()
                || !(is_package(dir) || is_target_dir(dir))
        });
    for entry in walk {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() && is_mim(entry.path()) {
                    files.push(entry.into_path());
                }
            }
            Err(e) => errors.push(std::io::Error::other(format!("{}: {e}", root.display()))),
        }
    }
    files.sort();
    (files, errors)
}

fn is_mim(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("mim")
}

/// Whether `dir` holds a cargo package, not just a workspace.
fn is_package(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("Cargo.toml"))
        .is_ok_and(|toml| toml.lines().any(|line| line.trim() == "[package]"))
}

/// Whether `dir` is a cargo target dir, going by the `CACHEDIR.TAG` cargo writes at its root.
pub(crate) fn is_target_dir(dir: &Path) -> bool {
    dir.join("CACHEDIR.TAG").is_file()
}
