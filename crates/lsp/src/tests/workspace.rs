use std::path::PathBuf;

use super::utils::{open, tree, update};
use crate::{host_api::Source, workspace::Workspace};

/// Loose scripts in `tools`, above a folder of scripts and a cargo package.
fn tools(name: &str) -> PathBuf {
    tree(
        name,
        &[
            ("tools/loose.mim", "let x = 1;"),
            (
                "tools/fodder/main.mim",
                "use w;
                 let y: int = w::two();",
            ),
            (
                "tools/fodder/w.mim",
                "module @;
                 pub fn two() -> int { 2 }",
            ),
            ("tools/docs/Cargo.toml", "[package]"),
            ("tools/docs/scripts/main.mim", "let z = 3;"),
        ],
    )
}

#[test]
fn nested_script_in_package_sees_modules_above() {
    let root = tree(
        "package",
        &[
            ("pkg/Cargo.toml", "[package]"),
            ("pkg/scripts/a.mim", "let x = 1;"),
            (
                "pkg/scripts/m.mim",
                "module @;
                 pub fn one() -> int { 1 }",
            ),
            (
                "pkg/scripts/enemies/b.mim",
                "use m;
                 let x: int = m::one();",
            ),
        ],
    );
    let script = root.join("pkg/scripts/enemies/b.mim");
    let mut workspace = Workspace::new(Source::Off);
    let changed = open(&mut workspace, &script);
    assert!(changed.contains(&(script.clone(), 0)), "{changed:?}");
    assert!(changed.iter().all(|(_, count)| *count == 0), "{changed:?}");
    assert_eq!(
        workspace.root_of(&root.join("pkg/scripts/enemies")),
        (root.join("pkg/scripts"), Some(root.join("pkg")))
    );
    assert!(workspace.analysis(&script).is_some());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn loose_script_roots() {
    let root = tools("roots");
    let workspace = Workspace::new(Source::Off);
    let root_of = |dir: &str| workspace.root_of(&root.join(dir));
    assert_eq!(root_of("tools"), (root.join("tools"), None));
    assert_eq!(root_of("tools/fodder"), (root.join("tools/fodder"), None));
    assert_eq!(
        root_of("tools/docs/scripts"),
        (
            root.join("tools/docs/scripts"),
            Some(root.join("tools/docs"))
        )
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn loose_script_leaves_nested_projects_alone() {
    let root = tools("nested");
    let loose = root.join("tools/loose.mim");
    let mut workspace = Workspace::new(Source::Off);
    open(&mut workspace, &loose);
    for path in [
        "tools/fodder/main.mim",
        "tools/fodder/w.mim",
        "tools/docs/scripts/main.mim",
    ] {
        let changed = open(&mut workspace, &root.join(path));
        assert!(changed.iter().all(|(_, count)| *count == 0), "{changed:?}");
    }

    assert_eq!(
        update(&mut workspace, &loose, Some("let x = 2;".into())),
        []
    );
    let project = |path: &str| workspace.project(&root.join(path)).unwrap();
    assert!(std::ptr::eq(
        project("tools/fodder/main.mim"),
        project("tools/fodder/w.mim")
    ));
    assert!(!std::ptr::eq(
        project("tools/fodder/main.mim"),
        project("tools/loose.mim")
    ));
    assert!(!std::ptr::eq(
        project("tools/docs/scripts/main.mim"),
        project("tools/loose.mim")
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn closing_the_last_file_clears_its_diagnostics() {
    let root = tree("close", &[("scripts/bad.mim", r#"let x: int = "no";"#)]);
    let bad = root.join("scripts/bad.mim");
    let mut workspace = Workspace::new(Source::Off);
    assert_eq!(open(&mut workspace, &bad), [(bad.clone(), 1)]);
    assert_eq!(update(&mut workspace, &bad, None), [(bad.clone(), 0)]);
    assert!(workspace.project(&bad).is_none());
    std::fs::remove_dir_all(root).unwrap();
}
