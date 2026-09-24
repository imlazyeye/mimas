use std::path::{Path, PathBuf};

use api::Library;

use crate::{Directory, Modules};

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
    let directory = Directory::load(&files, Path::new("p"), &Library::new());
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
    assert!(directory.out_of_place_scripts.is_empty());
}

#[test]
fn scripts_isolated() {
    let files = [
        file("p/a.mim", "fn shared() -> int { 1 }"),
        file("p/b.mim", "fn shared() -> int { 2 }"),
        file("p/c.mim", "let x: int = shared();"),
    ];
    let directory = Directory::load(&files, Path::new("p"), &Library::new());
    let clean: Vec<bool> = directory
        .scripts
        .iter()
        .map(|script| solve(&directory, script).errors.is_empty())
        .collect();
    assert_eq!(clean, [true, true, false]);
}

#[test]
fn out_of_place_script() {
    let files = [
        file("p/a.mim", ""),
        file("p/sub/b.mim", ""),
        file("p/m.mim", "module @;"),
    ];
    let directory = Directory::load(&files, Path::new("p"), &Library::new());
    let scripts: Vec<&Path> = directory
        .scripts
        .iter()
        .map(|file| file.0.as_path())
        .collect();
    assert_eq!(scripts, [Path::new("p/a.mim")]);
    assert_eq!(directory.out_of_place_scripts, [Path::new("p/sub/b.mim")]);
    assert_eq!(directory.module_files.len(), 1);
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
    let directory = Directory::load(&files, Path::new("p"), &Library::new());
    assert_eq!(directory.modules.errors.len(), 1);
    assert!(solve(&directory, directory.scripts[0]).errors.is_empty());
}
