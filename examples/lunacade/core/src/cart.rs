use std::{collections::BTreeMap, io, path::Path};

use serde::{Deserialize, Serialize};

use crate::{Diagnostic, SHEET_SIZE};

/// A cart's files by path.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cart {
    pub files: BTreeMap<String, String>,
}

/// A validated cart with the pieces needed to run it in the machine.
pub struct CartBuild<'a> {
    pub sheet: [u8; SHEET_SIZE * SHEET_SIZE],
    pub sources: Vec<(&'a str, &'a str)>,
    pub script: &'a str,
}

impl Cart {
    /// Reads a cart from a folder.
    pub fn from_dir(dir: &Path) -> io::Result<Self> {
        let mut files = BTreeMap::new();
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if (name == "sprites.txt" || name.ends_with(".mim")) && path.is_file() {
                files.insert(name.to_owned(), std::fs::read_to_string(&path)?);
            }
        }
        Ok(Self { files })
    }

    /// The `.mim` files that open with `module`, in path order.
    pub fn modules(&self) -> impl Iterator<Item = (&str, &str)> {
        self.mim_files(true)
    }

    /// The `.mim` files that aren't modules, in path order.
    pub fn scripts(&self) -> impl Iterator<Item = (&str, &str)> {
        self.mim_files(false)
    }

    /// The cart's script as its path and text, unless it has none or several.
    pub fn script(&self) -> Option<(&str, &str)> {
        let mut scripts = self.scripts();
        scripts.next().filter(|_| scripts.next().is_none())
    }

    fn mim_files(&self, modules: bool) -> impl Iterator<Item = (&str, &str)> {
        let files = self.files.iter().filter(move |(path, text)| {
            path.ends_with(".mim") && parse::lex::is_module(text) == modules
        });
        files.map(|(path, text)| (path.as_str(), text.as_str()))
    }

    /// Checks everything about the cart that doesn't need the compiler, and gives every problem
    /// it finds. A cart has one script among its `.mim` files. A `sprites.txt` is up to 128 lines
    /// of up to 128 hex digits, and the lines it doesn't have are black.
    pub fn validate(&self) -> Result<CartBuild<'_>, Vec<Diagnostic>> {
        fn sheet(text: &str) -> Result<[u8; SHEET_SIZE * SHEET_SIZE], Diagnostic> {
            let mut sheet = [0; SHEET_SIZE * SHEET_SIZE];
            let mut offset = 0;
            for (y, chunk) in text.split_inclusive('\n').enumerate() {
                let line = chunk.trim_end_matches(['\r', '\n']);
                if y >= SHEET_SIZE && !line.is_empty() {
                    let span = offset..offset + line.len();
                    let message = format!("a sprite sheet has {SHEET_SIZE} lines at most");
                    return Err(Diagnostic::at("sprites.txt", text, span, message));
                }
                for (x, (at, c)) in line.char_indices().enumerate() {
                    let span = offset + at..offset + at + c.len_utf8();
                    let message = if x >= SHEET_SIZE {
                        format!("a sprite sheet line has {SHEET_SIZE} digits at most")
                    } else if let Some(color) = c.to_digit(16) {
                        sheet[y * SHEET_SIZE + x] = color as u8;
                        continue;
                    } else {
                        format!("`{c}` is not a hex digit")
                    };
                    let file = "sprites.txt";
                    return Err(Diagnostic::at(file, text, span, message));
                }
                offset += chunk.len();
            }
            Ok(sheet)
        }

        let mut problems = Vec::new();
        let scripts: Vec<_> = self.scripts().collect();
        match scripts[..] {
            [_] => {}
            [] => problems.push(Diagnostic::at("", "", 0..0, "a cart has no script to run")),
            _ => {
                let names: Vec<_> = scripts.iter().map(|(path, _)| *path).collect();
                let message = format!("a cart has several scripts: {}", names.join(", "));
                for (path, text) in &scripts {
                    problems.push(Diagnostic::at(path, text, 0..0, &message));
                }
            }
        }
        let sheet = match self.files.get("sprites.txt") {
            Some(text) => sheet(text).map_err(|problem| problems.push(problem)).ok(),
            None => Some([0; SHEET_SIZE * SHEET_SIZE]),
        };
        match (sheet, &scripts[..]) {
            (Some(sheet), &[(script, _)]) if problems.is_empty() => Ok(CartBuild {
                sheet,
                sources: self.sources(),
                script,
            }),
            _ => Err(problems),
        }
    }

    /// The `.mim` files as the paths and texts `Vm::compile_files` takes.
    pub fn sources(&self) -> Vec<(&str, &str)> {
        self.modules().chain(self.script()).collect()
    }
}
