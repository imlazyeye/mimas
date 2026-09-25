use std::path::{Path, PathBuf};

use api::Library;

use crate::{Directory, Modules, mim_files};

fn file(path: &str, text: &str) -> (PathBuf, String) {
    (PathBuf::from(path), text.to_owned())
}

fn solve(directory: &Directory, (path, text): &(PathBuf, String)) -> Modules {
    directory.modules.load([(path, text.as_str())])
}

#[test]
fn scripts_see_all_modules() {
    let files = [
        file(
            "p/m.mim",
            "
            module @;
            pub fn one() -> int { 1 }
        ",
        ),
        file(
            "p/sub/n.mim",
            "
            module @;
            pub fn two() -> int { 2 }
        ",
        ),
        file(
            "p/a.mim",
            "
            use m;
            use n;
            let x: int = m::one() + n::two();
        ",
        ),
    ];
    let directory = Directory::load(&files, &Library::new());
    assert!(
        directory.modules.errors.is_empty(),
        "{:?}",
        directory.modules.errors
    );
    let [script] = directory.scripts.as_slice() else {
        panic!("one script");
    };
    assert_eq!(script.0, Path::new("p/a.mim"));
    let loaded = solve(&directory, script);
    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
}

#[test]
fn scripts_isolated() {
    let files = [
        file("p/a.mim", "fn shared() -> int { 1 }"),
        file("p/b.mim", "fn shared() -> int { 2 }"),
        file("p/c.mim", "let x: int = shared();"),
    ];
    let directory = Directory::load(&files, &Library::new());
    let clean: Vec<bool> = directory
        .scripts
        .iter()
        .map(|script| solve(&directory, script).errors.is_empty())
        .collect();
    assert_eq!(clean, [true, true, false]);
}

#[test]
fn nested_script_sees_all_modules() {
    let files = [
        file("p/a.mim", ""),
        file(
            "p/sub/b.mim",
            "use m;
             let x: int = m::one();",
        ),
        file(
            "p/m.mim",
            "module @;
             pub fn one() -> int { 1 }",
        ),
    ];
    let directory = Directory::load(&files, &Library::new());
    let scripts: Vec<&Path> = directory
        .scripts
        .iter()
        .map(|file| file.0.as_path())
        .collect();
    assert_eq!(scripts, [Path::new("p/a.mim"), Path::new("p/sub/b.mim")]);
    let loaded = solve(&directory, directory.scripts[1]);
    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
}

#[test]
fn broken_module_cuts_off_script() {
    let files = [
        file(
            "p/m.mim",
            "
            module @;
            pub fn f() -> int { \"no\" }
        ",
        ),
        file("p/a.mim", "let x: str = 1;"),
    ];
    let directory = Directory::load(&files, &Library::new());
    assert_eq!(directory.modules.errors.len(), 1);
    assert!(solve(&directory, directory.scripts[0]).errors.is_empty());
}

#[test]
fn walk_skips_packages_and_target_dirs() {
    let root = std::env::temp_dir().join(format!("mimas-walk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (path, text) in [
        ("a.mim", ""),
        ("sub/b.mim", ""),
        ("workspace/Cargo.toml", "[workspace]"),
        ("workspace/c.mim", ""),
        ("pkg/Cargo.toml", "[package]"),
        ("pkg/d.mim", ""),
        ("target/CACHEDIR.TAG", ""),
        ("target/e.mim", ""),
    ] {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let walked = mim_files(&root, true).0;
    let expected = ["a.mim", "sub/b.mim", "workspace/c.mim"].map(|path| root.join(path));
    assert_eq!(walked, expected);
    assert_eq!(
        mim_files(&root.join("pkg"), true).0,
        [root.join("pkg/d.mim")]
    );
    std::fs::remove_dir_all(root).unwrap();
}
