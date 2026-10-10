//! Embeds the cart `LUNACADE_CART` names, with the files `Cart::from_dir` would read, so the
//! binary is that game alone. The path is relative to `examples/lunacade`. Without it the binary
//! runs the cart folder it's given.
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
                files.push((key, path));
            }
        }
    }

    println!("cargo:rerun-if-env-changed=LUNACADE_CART");
    let mut out = String::from("const EMBEDDED: Option<(&str, &[(&str, &str)])> = ");
    match env::var("LUNACADE_CART") {
        Ok(cart) => {
            let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(&cart);
            let dir = dir
                .canonicalize()
                .unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
            let mut files = Vec::new();
            walk(&dir, "", &mut files);
            files.sort();
            out += &format!(
                "Some(({:?}, &[\n",
                dir.file_name().unwrap().to_str().unwrap()
            );
            for (key, path) in files {
                out += &format!("    ({key:?}, include_str!({:?})),\n", path.display());
            }
            out += "]));\n";
        }
        Err(_) => out += "None;\n",
    }
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("cart.rs"),
        out,
    )
    .unwrap();
}
