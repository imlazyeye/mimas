//! Lists the carts in `examples/lunacade/carts` for `carts.rs`, each with the files
//! `Cart::from_dir` would read, so the web build ships whatever the folder holds.
use std::{env, fs, path::Path};

fn main() {
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
        println!("cargo:rerun-if-changed={}", dir.display());
        let id = dir.file_name().unwrap().to_str().unwrap();
        out += &format!("    ({id:?}, &[\n");
        let mut files: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_file())
            .collect();
        files.sort();
        for file in files {
            let name = file.file_name().unwrap().to_str().unwrap();
            if name == "sprites.txt" || name.ends_with(".mim") {
                let path = file.canonicalize().unwrap();
                out += &format!("        ({name:?}, include_str!({:?})),\n", path.display());
            }
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
