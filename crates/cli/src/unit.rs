use solve::Directory;
use std::path::{Path, PathBuf};

/// The files of the project a path names: a directory, or a file with the directory it sits in.
pub struct Unit {
    pub root: PathBuf,
    pub file: Option<PathBuf>,
    pub files: Vec<(PathBuf, String)>,
    pub io_errors: Vec<std::io::Error>,
}

impl Unit {
    pub fn new(path: &Path) -> Self {
        let file = path.is_file().then(|| path.to_path_buf());
        let root = match &file {
            Some(file) => solve::dir_of(file),
            None => path,
        }
        .to_path_buf();
        let (mut paths, mut io_errors) = solve::mim_files(&root, true);
        // a file is taken as given, whatever its extension, in place of its walked twin
        if let Some(file) = &file {
            paths.retain(|walked| {
                solve::dir_of(walked) != root.as_path() || walked.file_name() != file.file_name()
            });
            paths.insert(0, file.clone());
        }

        let mut files = vec![];
        for file_path in paths {
            match std::fs::read_to_string(&file_path) {
                Ok(source) => files.push((file_path, source)),
                Err(e) => io_errors.push(std::io::Error::new(
                    e.kind(),
                    format!("{}: {e}", file_path.display()),
                )),
            }
        }
        Self {
            root,
            file,
            files,
            io_errors,
        }
    }

    /// The scripts the path names: the file itself, or every one at the top of the directory.
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
