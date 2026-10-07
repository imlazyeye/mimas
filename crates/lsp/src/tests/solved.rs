use std::path::{Path, PathBuf};

use lsp_types::{Contents, Position};

use super::utils::{open, tree};
use crate::{host_api::HostApi, workspace::Workspace};

/// A module with a documented `one`, and two scripts that call it.
fn scripts(name: &str, b: &str) -> PathBuf {
    tree(
        name,
        &[
            (
                "m.mim",
                "module @;
                 /// The first number.
                 ///
                 /// ```mimas no_run
                 /// # use m;
                 /// let x = m::one();
                 /// ```
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
    let mut workspace = Workspace::new(HostApi::Off);
    open(&mut workspace, &a);
    let project = workspace.project(&a).unwrap();
    let references = project
        .references(&m, at(&m, "one() -> int"), true)
        .unwrap();
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
    let mut workspace = Workspace::new(HostApi::Off);
    open(&mut workspace, &a);
    let project = workspace.project(&a).unwrap();
    let references = project
        .references(&m, at(&m, "one() -> int"), true)
        .unwrap();
    assert_eq!(references.len(), 2, "{references:?}");
    let refused = project.rename(&a, at(&a, "one"), "uno").unwrap_err();
    assert!(refused.contains("check cleanly"), "{refused}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hover_shows_a_module_function_and_its_doc() {
    let root = scripts("hover", "let y = 2;");
    let a = root.join("a.mim");
    let mut workspace = Workspace::new(HostApi::Off);
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
    assert!(
        content.value.contains("let x = m::one();") && !content.value.contains("use m;"),
        "{}",
        content.value
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn each_path_resolves_its_member() {
    let root = tree(
        "each",
        &[(
            "a.mim",
            "pact Shape {
                 fn sides() -> int;
             }
             struct Square;
             impl Shape for Square {
                 fn sides() -> int {
                     4
                 }
             }
             let all = Shape::*::sides();",
        )],
    );
    let a = root.join("a.mim");
    let mut workspace = Workspace::new(HostApi::Off);
    open(&mut workspace, &a);
    let project = workspace.project(&a).unwrap();
    let references = project
        .references(&a, at(&a, "sides() -> int;"), true)
        .unwrap();
    assert_eq!(references.len(), 2, "{references:?}");
    let hover = workspace
        .analysis(&a)
        .unwrap()
        .hover(&a, at(&a, "sides();"))
        .unwrap();
    let Contents::MarkupContent(content) = hover.contents else {
        panic!("{:?}", hover.contents);
    };
    assert!(
        content.value.contains("fn sides() -> [int]"),
        "{}",
        content.value
    );
    std::fs::remove_dir_all(root).unwrap();
}

macro_rules! definition_test {
    ($name:ident, $source:expr, $($usage:expr => $declaration:expr),+ $(,)?) => {
        #[test]
        fn $name() {
            let root = tree(stringify!($name), &[("a.mim", $source)]);
            let path = root.join("a.mim");
            let mut workspace = Workspace::new(HostApi::Off);
            assert_eq!(open(&mut workspace, &path), vec![(path.clone(), 0)]);
            let analysis = workspace.analysis(&path).unwrap();
            $(
                let definition = analysis.definition(&path, at(&path, $usage)).unwrap();
                assert_eq!(definition.uri.to_file_path().unwrap(), path);
                assert_eq!(definition.range.start, at(&path, $declaration), "{}", $usage);
            )+
            std::fs::remove_dir_all(root).unwrap();
        }
    };
}

definition_test!(
    pattern_field_keys_refer_to_fields,
    "struct Pair { first: int, second: int }
     let pair = Pair { first = 1, second = 2 };
     let whole @ Pair { first = chosen, second } = pair else loop {};
     whole.first + chosen;",
    "first = chosen" => "first: int",
    "second } =" => "second } =",
    "chosen;" => "chosen, second",
    "whole.first" => "whole @",
);

definition_test!(
    pattern_or_bindings_share_the_first_declaration,
    "enum E { A(int), B(int) }
     match E::B(2) { E::A(n) | E::B(n) => n + 1 }",
    "n) =>" => "n) |",
    "n + 1" => "n) |",
);

definition_test!(
    pattern_or_shorthand_refers_to_the_local,
    "enum E { A { value: int }, B { value: int } }
     let e = E::B { value = 2 };
     match e { E::A { value } | E::B { value } => value + 1 }",
    "value } =>" => "value } |",
    "value + 1" => "value } |",
);
