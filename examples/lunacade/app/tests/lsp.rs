use std::collections::BTreeMap;

use lunacade::{check_cart, definition, highlights, hover, inlay_hints, prepare_rename, rename};
use serde_json::{Value, from_str, json};

fn cart(files: &[(&str, &str)]) -> String {
    let files: BTreeMap<_, _> = files.iter().copied().collect();
    json!({ "files": files }).to_string()
}

fn checked(files: &[(&str, &str)]) -> Value {
    from_str(&check_cart(&cart(files)).unwrap()).unwrap()
}

#[test]
fn check_matches_the_core() {
    let files = [
        ("main.mim", "let a: int = \"x\";\nluna::draw(|| {});"),
        (
            "util.mim",
            "module @;\npub fn bad() -> int {\n    \"text\"\n}",
        ),
    ];
    let core = lunacade_core::Cart {
        files: files
            .iter()
            .map(|(path, text)| (path.to_string(), text.to_string()))
            .collect(),
    };
    assert_eq!(
        checked(&files),
        serde_json::to_value(lunacade_core::check(&core)).unwrap()
    );
    let list = checked(&files);
    assert_eq!(list[0]["file"], "util.mim");
    assert_eq!(list[0]["message"], "mismatched types");
}

#[test]
fn invalid_cart_checks_every_script_and_clears_analysis() {
    assert_eq!(
        checked(&[("main.mim", "luna::draw(|| gfx::clear(Color::Navy));")]),
        json!([])
    );
    assert!(hover("main.mim", 0, 18).is_some());
    let files = [
        ("main.mim", "let a: int = \"x\";"),
        ("other.mim", "let b: int = \"y\";"),
    ];
    let core = lunacade_core::Cart {
        files: files
            .iter()
            .map(|(path, text)| (path.to_string(), text.to_string()))
            .collect(),
    };
    let found = checked(&files);
    assert_eq!(
        found,
        serde_json::to_value(lunacade_core::check(&core)).unwrap()
    );
    assert_eq!(found.as_array().unwrap().len(), 4);
    assert!(hover("main.mim", 0, 18).is_none());
    assert!(prepare_rename("main.mim", 0, 4).is_none());
}

#[test]
fn hover_shows_the_docs() {
    assert_eq!(
        checked(&[("main.mim", "luna::draw(|| gfx::clear(Color::Navy));")]),
        json!([])
    );
    let found: Value = from_str(&hover("main.mim", 0, 18).unwrap()).unwrap();
    let text = found["contents"]["value"].as_str().unwrap();
    assert!(text.contains("fn clear(_: Color)"), "{text}");
    assert!(text.contains("Fills the whole screen"), "{text}");
    assert!(found["range"]["start"]["character"].as_u64().unwrap() <= 18);
    assert!(found["range"]["end"]["character"].as_u64().unwrap() > 18);
    assert!(hover("missing.mim", 0, 0).is_none());
}

#[test]
fn definition_and_rename_cross_files() {
    let files = [
        (
            "main.mim",
            "use util;\nprint(util::double(2));\nluna::draw(|| {});",
        ),
        (
            "lib/util.mim",
            "module @;\npub fn double(n: int) -> int {\n    n * 2\n}",
        ),
    ];
    assert_eq!(checked(&files), json!([]));
    let place: Value = from_str(&definition("main.mim", 1, 12).unwrap()).unwrap();
    assert_eq!(place["file"], "lib/util.mim");
    assert_eq!(place["range"]["start"]["line"], 1);
    assert_eq!(place["range"]["start"]["character"], 7);

    assert!(prepare_rename("main.mim", 1, 12).is_some());
    let renamed: Value = from_str(&rename("main.mim", 1, 12, "twice")).unwrap();
    let changes = &renamed["changes"];
    assert_eq!(changes["main.mim"].as_array().unwrap().len(), 1);
    assert_eq!(changes["lib/util.mim"][0]["newText"], "twice");
    assert_eq!(changes["lib/util.mim"][0]["range"]["start"]["line"], 1);

    let refused: Value = from_str(&rename("main.mim", 1, 12, "not a name")).unwrap();
    assert!(refused["error"].as_str().unwrap().contains("valid"));
    assert!(
        prepare_rename("main.mim", 1, 6).is_none(),
        "print is built in"
    );
}

#[test]
fn hints_and_highlights() {
    assert_eq!(
        checked(&[("main.mim", "let n = 1;\nlet m = n + n;\nluna::draw(|| {});")]),
        json!([])
    );
    let hints: Value = from_str(&inlay_hints("main.mim").unwrap()).unwrap();
    assert_eq!(hints.as_array().unwrap().len(), 2);
    assert_eq!(hints[0]["label"], ": int");
    assert_eq!(hints[0]["position"], json!({ "line": 0, "character": 5 }));
    let uses: Value = from_str(&highlights("main.mim", 0, 4).unwrap()).unwrap();
    assert_eq!(uses.as_array().unwrap().len(), 3);
}
