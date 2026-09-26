use api::Project;

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
    let walked = Project::of(&root).files().0;
    let expected = ["a.mim", "sub/b.mim", "workspace/c.mim"].map(|path| root.join(path));
    assert_eq!(walked, expected);
    assert_eq!(
        Project::of(&root.join("pkg")).files().0,
        [root.join("pkg/d.mim")]
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_folder_is_its_own_root_inside_a_package() {
    let root = std::env::temp_dir().join(format!("mimas-folder-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (path, text) in [
        ("Cargo.toml", "[package]"),
        ("scripts/a.mim", ""),
        ("scripts/enemies/b.mim", ""),
    ] {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    assert_eq!(
        Project::of(&root.join("scripts/enemies")),
        Project {
            root: root.join("scripts/enemies"),
            recursive: true,
            package: Some(root.clone()),
        }
    );
    assert_eq!(
        Project::of(&root.join("scripts/enemies/b.mim")).root,
        root.join("scripts")
    );
    std::fs::remove_dir_all(root).unwrap();
}
