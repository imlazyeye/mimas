use std::path::{Path, PathBuf};

use crate::workspace::Workspace;

/// Writes `files` into a fresh folder in the temp dir, with a `.git` at its top.
pub(crate) fn tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("mimas-lsp-{name}-{}", std::process::id()));
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

/// Each file whose diagnostics changed with the update, and how many it has now.
pub(crate) fn update(
    workspace: &mut Workspace,
    path: &Path,
    text: Option<String>,
) -> Vec<(PathBuf, usize)> {
    let mut changes: Vec<_> = workspace
        .update(path, text)
        .0
        .into_iter()
        .map(|(uri, diagnostics)| (uri.to_file_path().unwrap(), diagnostics.len()))
        .collect();
    changes.sort();
    changes
}
