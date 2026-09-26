use std::{
    fs::File,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use api::{ApiConstant, ApiEntry, Library, Manifest, Project};
use shared::{Literal, Ty};

use super::utils::{open, tree};
use crate::{host_api::HostApi, workspace::Workspace};

/// A cargo package with a binary and an example, and a script that uses `host::SPEED`.
fn package(name: &str) -> PathBuf {
    tree(
        name,
        &[
            (
                "Cargo.toml",
                r#"[package]
                   name = "game"
                   version = "0.1.0"
                   edition = "2024""#,
            ),
            ("src/main.rs", "fn main() {}"),
            ("examples/editor.rs", "fn main() {}"),
            ("scripts/speed.mim", "let x: float = host::SPEED;"),
        ],
    )
}

/// Writes the manifest `binary` would, with `host::<constant>` in it, dated `age` from now.
fn write(root: &Path, binary: &str, constant: &str, age: Duration) {
    let mut library = Library::new();
    library.constant(ApiConstant {
        name: constant.into(),
        module: vec!["host".into()],
        recv_ty: None,
        ty: Ty::Float,
        value: Literal::Float(1.0),
        doc: String::new(),
    });
    let path = Manifest::path(&root.join("target"), binary);
    Manifest::new(library).write(&path).unwrap();
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(SystemTime::now() - age)
        .unwrap();
}

fn has(library: &Library<()>, constant: &str) -> bool {
    library
        .natives()
        .any(|(_, entry)| matches!(entry, ApiEntry::Constant(c) if c.name == constant))
}

#[test]
fn newest_binary_manifest_wins() {
    let root = package("newest");
    let project = Project::of(&root.join("scripts/speed.mim"));
    let (library, missing) = HostApi::Packages.library(&project);
    assert!(!has(&library, "SPEED"));
    assert!(missing.is_some());

    write(&root, "game", "SPEED", Duration::from_secs(60));
    assert!(has(&HostApi::Packages.library(&project).0, "SPEED"));

    write(&root, "editor", "ZOOM", Duration::ZERO);
    let (library, missing) = HostApi::Packages.library(&project);
    assert!(has(&library, "ZOOM"));
    assert_eq!(missing, None);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn running_the_host_rechecks_its_scripts() {
    let root = package("rerun");
    let script = root.join("scripts/speed.mim");
    let mut workspace = Workspace::new(HostApi::Packages);
    // the unknown `host::SPEED`, and a note that the host hasn't run yet
    assert_eq!(open(&mut workspace, &script), [(script.clone(), 2)]);

    write(&root, "game", "SPEED", Duration::ZERO);
    assert_eq!(open(&mut workspace, &script), [(script.clone(), 0)]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn an_unreadable_manifest_says_how_to_write_it() {
    let json = format!(r#"{{"version": "{}", "library": []}}"#, api::VERSION);
    let root = tree("unreadable", &[("api.json", &json)]);
    let (_, problem) = HostApi::Manifest(root.join("api.json")).library(&Project::of(&root));
    let problem = problem.unwrap();
    assert!(
        problem.contains("isn't a host API mimas can read"),
        "{problem}"
    );
    assert!(problem.contains("`mimas::write_api`"), "{problem}");
    std::fs::remove_dir_all(root).unwrap();
}
