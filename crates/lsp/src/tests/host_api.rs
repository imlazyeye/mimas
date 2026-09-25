use std::{
    fs::File,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, SystemTime},
};

use api::{ApiConstant, ApiEntry, Library, ManifestRef};
use shared::{Literal, Ty};

use super::utils::{open, tree};
use crate::{
    host_api::{HostApis, Source},
    workspace::Workspace,
};

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
    let manifest = ManifestRef {
        version: api::VERSION,
        library: &library,
    };
    let path = api::manifest_file(&root.join("target"), binary);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
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
    let mut hosts = HostApis::new(Source::Packages);
    assert!(!has(&hosts.library(Some(&root)), "SPEED"));
    assert_eq!(hosts.messages.len(), 1, "{:?}", hosts.messages);

    write(&root, "game", "SPEED", Duration::from_secs(60));
    let game = hosts.library(Some(&root));
    assert!(has(&game, "SPEED"));
    assert!(Rc::ptr_eq(&game, &hosts.library(Some(&root))));

    write(&root, "editor", "ZOOM", Duration::ZERO);
    assert!(has(&hosts.library(Some(&root)), "ZOOM"));
    assert_eq!(hosts.messages.len(), 1, "{:?}", hosts.messages);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn running_the_host_rechecks_its_scripts() {
    let root = package("rerun");
    let script = root.join("scripts/speed.mim");
    let mut workspace = Workspace::new(Source::Packages);
    assert_eq!(open(&mut workspace, &script), [(script.clone(), 1)]);

    write(&root, "game", "SPEED", Duration::ZERO);
    assert_eq!(open(&mut workspace, &script), [(script.clone(), 0)]);
    std::fs::remove_dir_all(root).unwrap();
}
