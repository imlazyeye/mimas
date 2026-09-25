use std::path::{Path, PathBuf};

use lsp_types::{Contents, Position};

use super::utils::{open, tree};
use crate::{host_api::Source, workspace::Workspace};

/// A module with a documented `one`, and two scripts that call it.
fn scripts(name: &str, b: &str) -> PathBuf {
    tree(
        name,
        &[
            (
                "m.mim",
                "module @;
                 /// The first number.
                 pub fn one() -> int { 1 }",
            ),
            (
                "a.mim",
                "use m;
                 let x: int = m::one();",
            ),
            ("b.mim", b),
        ],
    )
}

/// Where `needle` first appears in the file at `path`.
fn at(path: &Path, needle: &str) -> Position {
    let text = std::fs::read_to_string(path).unwrap();
    let before = &text[..text.find(needle).unwrap()];
    let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
    Position {
        line: before.matches('\n').count() as u32,
        character: (before.len() - line_start) as u32,
    }
}

#[test]
fn references_and_rename_reach_every_script() {
    let root = scripts(
        "reach",
        "use m;
         let y: int = m::one() + m::one();",
    );
    let (a, m) = (root.join("a.mim"), root.join("m.mim"));
    let mut workspace = Workspace::new(Source::Off);
    open(&mut workspace, &a);
    let project = workspace.project(&a).unwrap();
    let references = project.references(&m, at(&m, "one"), true).unwrap();
    assert_eq!(references.len(), 4, "{references:?}");
    let edit = project.rename(&a, at(&a, "one"), "uno").unwrap();
    let edits: usize = edit.changes.unwrap().values().map(Vec::len).sum();
    assert_eq!(edits, 4);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_script_that_does_not_check_has_no_references_to_add() {
    let root = scripts(
        "broken",
        r#"use m;
           let y: int = m::one();
           let z: int = "no";"#,
    );
    let (a, m) = (root.join("a.mim"), root.join("m.mim"));
    let mut workspace = Workspace::new(Source::Off);
    open(&mut workspace, &a);
    let project = workspace.project(&a).unwrap();
    let references = project.references(&m, at(&m, "one"), true).unwrap();
    assert_eq!(references.len(), 2, "{references:?}");
    let refused = project.rename(&a, at(&a, "one"), "uno").unwrap_err();
    assert!(refused.contains("check cleanly"), "{refused}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hover_shows_a_module_function_and_its_doc() {
    let root = scripts("hover", "let y = 2;");
    let a = root.join("a.mim");
    let mut workspace = Workspace::new(Source::Off);
    open(&mut workspace, &a);
    let hover = workspace
        .analysis(&a)
        .unwrap()
        .hover(&a, at(&a, "one"))
        .unwrap();
    let Contents::MarkupContent(content) = hover.contents else {
        panic!("{:?}", hover.contents);
    };
    assert!(
        content.value.contains("fn one() -> int"),
        "{}",
        content.value
    );
    assert!(
        content.value.contains("The first number."),
        "{}",
        content.value
    );
    std::fs::remove_dir_all(root).unwrap();
}
