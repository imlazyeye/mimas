use api::Project;

use super::utils::{open, tree, update};
use crate::{host_api::HostApi, workspace::Workspace};

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
            ("pkg/tests/t.mim", "let y = 2;"),
        ],
    );
    let script = root.join("pkg/scripts/enemies/b.mim");
    let mut workspace = Workspace::new(HostApi::Off);
    let changed = open(&mut workspace, &script);
    assert!(changed.contains(&(script.clone(), 0)), "{changed:?}");
    assert!(changed.iter().all(|(_, count)| *count == 0), "{changed:?}");
    assert_eq!(
        Project::of(&script),
        Project {
            root: root.join("pkg/scripts"),
            recursive: true,
            package: Some(root.join("pkg")),
        }
    );
    assert_eq!(
        Project::of(&root.join("pkg/tests/t.mim")).root,
        root.join("pkg/tests")
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn loose_scripts_share_the_highest_folder() {
    let root = tree(
        "loose",
        &[
            (
                "tools/loose.mim",
                "use w;
                 let x: int = w::two();",
            ),
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
    );
    let loose = root.join("tools/loose.mim");
    let fodder = root.join("tools/fodder/main.mim");
    let docs = root.join("tools/docs/scripts/main.mim");
    assert_eq!(
        Project::of(&loose),
        Project {
            root: root.join("tools"),
            recursive: true,
            package: None,
        }
    );
    assert_eq!(Project::of(&fodder).root, root.join("tools"));
    assert_eq!(
        Project::of(&docs),
        Project {
            root: root.join("tools/docs/scripts"),
            recursive: true,
            package: Some(root.join("tools/docs")),
        }
    );

    let mut workspace = Workspace::new(HostApi::Off);
    let changed = open(&mut workspace, &loose);
    assert_eq!(changed.len(), 3, "{changed:?}");
    assert!(changed.iter().all(|(_, count)| *count == 0), "{changed:?}");
    assert_eq!(open(&mut workspace, &docs), [(docs.clone(), 0)]);

    let broken = Some(r#"let x: int = "no";"#.to_owned());
    let changed = update(&mut workspace, &loose, broken);
    assert!(changed.contains(&(loose.clone(), 1)), "{changed:?}");
    assert!(
        !changed.iter().any(|(file, _)| *file == docs),
        "{changed:?}"
    );
    let project = |path| workspace.project(path).unwrap();
    assert!(std::ptr::eq(project(&fodder), project(&loose)));
    assert!(!std::ptr::eq(project(&docs), project(&loose)));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_file_outside_any_repository_takes_its_folder_alone() {
    let root = tree(
        "lone",
        &[("x.mim", "let x = 1;"), ("sub/y.mim", "let y = 2;")],
    );
    std::fs::remove_dir(root.join(".git")).unwrap();
    let x = root.join("x.mim");
    assert_eq!(
        Project::of(&x),
        Project {
            root: root.clone(),
            recursive: false,
            package: None,
        }
    );
    let mut workspace = Workspace::new(HostApi::Off);
    assert_eq!(open(&mut workspace, &x), [(x.clone(), 0)]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_new_file_joins_its_project() {
    let root = tree("new", &[("scripts/a.mim", "let x = 1;")]);
    let a = root.join("scripts/a.mim");
    let b = root.join("scripts/b.mim");
    let mut workspace = Workspace::new(HostApi::Off);
    open(&mut workspace, &a);
    std::fs::write(&b, r#"let y: int = "no";"#).unwrap();
    assert_eq!(open(&mut workspace, &b), [(a.clone(), 0), (b.clone(), 1)]);
    assert!(workspace.analysis(&b).is_some());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn closing_the_last_file_clears_its_diagnostics() {
    let root = tree("close", &[("scripts/bad.mim", r#"let x: int = "no";"#)]);
    let bad = root.join("scripts/bad.mim");
    let mut workspace = Workspace::new(HostApi::Off);
    assert_eq!(open(&mut workspace, &bad), [(bad.clone(), 1)]);
    assert_eq!(update(&mut workspace, &bad, None), [(bad.clone(), 0)]);
    assert!(workspace.project(&bad).is_none());
    std::fs::remove_dir_all(root).unwrap();
}
