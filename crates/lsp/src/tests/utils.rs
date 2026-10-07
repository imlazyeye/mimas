use std::path::{Path, PathBuf};

use api::Project;

use mimas_lsp::workspace::Workspace;

/// Writes `files` into a fresh folder outside other repositories and packages, with a `.git` at its
/// top.
pub(crate) fn tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let bases = [
        Some(std::env::temp_dir()),
        #[cfg(unix)]
        Some(PathBuf::from("/var/tmp")),
        #[cfg(windows)]
        std::env::var_os("LOCALAPPDATA").map(|path| PathBuf::from(path).join("Temp")),
    ];
    let base = bases
        .into_iter()
        .flatten()
        .find(|base| {
            base.is_dir()
                && base.ancestors().all(|dir| !dir.join(".git").exists())
                && Project::of(base).package.is_none()
        })
        .expect("no temporary folder outside a repository or package");
    let root = base.join(format!("mimas-lsp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".git")).unwrap();
    for (path, text) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    root
}

/// Opens `path` with its text on disk.
pub(crate) fn open(workspace: &mut Workspace, path: &Path) -> Vec<(PathBuf, usize)> {
    update(
        workspace,
        path,
        Some(std::fs::read_to_string(path).unwrap()),
    )
}

/// Each file the update published diagnostics for, and how many it has now.
pub(crate) fn update(
    workspace: &mut Workspace,
    path: &Path,
    text: Option<String>,
) -> Vec<(PathBuf, usize)> {
    let mut changes: Vec<_> = workspace
        .update(path, text)
        .into_iter()
        .map(|(uri, diagnostics)| (uri.to_file_path().unwrap(), diagnostics.len()))
        .collect();
    changes.sort();
    changes
}
