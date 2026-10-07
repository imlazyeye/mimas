//! Lists the carts in `examples/lunacade/carts` for `carts.rs`, each with the files
//! `Cart::from_dir` would read, so the web build ships whatever the folders hold.
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    fn walk(dir: &Path, prefix: &str, files: &mut Vec<(String, PathBuf)>) {
        println!("cargo:rerun-if-changed={}", dir.display());
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_str().unwrap();
            let key = format!("{prefix}{name}");
            if path.is_dir() {
                if !name.starts_with('.') {
                    walk(&path, &format!("{key}/"), files);
                }
            } else if name.ends_with(".mim") || key == "sprites.txt" {
                files.push((key, path.canonicalize().unwrap()));
            }
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../carts");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut dirs: Vec<_> = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("{}: {error}", root.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();

    let mut out = String::from("pub(crate) const CARTS: &[(&str, &[(&str, &str)])] = &[\n");
    for dir in dirs {
        let id = dir.file_name().unwrap().to_str().unwrap();
        out += &format!("    ({id:?}, &[\n");
        let mut files = Vec::new();
        walk(&dir, "", &mut files);
        files.sort();
        for (key, path) in files {
            out += &format!("        ({key:?}, include_str!({:?})),\n", path.display());
        }
        out += "    ]),\n";
    }
    out += "];\n";
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("carts.rs"),
        out,
    )
    .unwrap();
}
