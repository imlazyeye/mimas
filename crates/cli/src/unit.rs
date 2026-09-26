use api::Project;
use solve::Directory;
use std::path::{Path, PathBuf};

/// The files of the project a path names: a directory, or a file with the rest of its project.
pub struct Unit {
    pub file: Option<PathBuf>,
    pub project: Project,
    pub files: Vec<(PathBuf, String)>,
    pub io_errors: Vec<std::io::Error>,
}

impl Unit {
    pub fn new(path: &Path) -> Self {
        // the project is found from the real path, but its files are shown from where mimas runs
        let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let cwd = std::env::current_dir()
            .and_then(std::fs::canonicalize)
            .unwrap_or_default();
        let shown = |path: &Path| path.strip_prefix(&cwd).unwrap_or(path).to_path_buf();

        let project = Project::of(&path);
        let file = path.is_file().then_some(path);
        let (mut paths, mut io_errors) = project.files();
        // a file is taken as given, whatever its extension, in place of its walked twin
        if let Some(file) = &file {
            paths.retain(|walked| walked != file);
            paths.insert(0, file.clone());
        }

        let mut files = vec![];
        for file_path in paths {
            match std::fs::read_to_string(&file_path) {
                Ok(source) => files.push((shown(&file_path), source)),
                Err(e) => io_errors.push(std::io::Error::new(
                    e.kind(),
                    format!("{}: {e}", shown(&file_path).display()),
                )),
            }
        }
        Self {
            file: file.as_deref().map(shown),
            project,
            files,
            io_errors,
        }
    }

    /// The scripts the path names: the file itself, or every one in the directory.
    pub fn scripts<'a>(&self, directory: &Directory<'a>) -> Vec<&'a (PathBuf, String)> {
        directory
            .scripts
            .iter()
            .copied()
            .filter(|(path, _)| self.file.as_ref().is_none_or(|file| file == path))
            .collect()
    }

    pub fn lines(&self) -> usize {
        self.files
            .iter()
            .map(|(_, source)| source.lines().count())
            .sum()
    }
}
