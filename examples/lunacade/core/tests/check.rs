mod runner;

use std::{ops::Range, path::Path};

use lunacade_core::{Cart, Diagnostic, DiagnosticKind, Input, Machine, check, check_with, library};
use runner::{cart, problems};

fn files(files: &[(&str, &str)]) -> Cart {
    let mut cart = cart("");
    cart.files.extend(
        files
            .iter()
            .map(|(path, text)| (path.to_string(), text.to_string())),
    );
    cart
}

fn one(cart: &Cart) -> Diagnostic {
    let mut found = check(cart);
    assert_eq!(found.len(), 1, "{found:?}");
    found.remove(0)
}

fn span(problem: &Diagnostic) -> (u32, u32, u32, u32) {
    (problem.line, problem.col, problem.end_line, problem.end_col)
}

fn at(text: &str, span: Range<usize>) -> Diagnostic {
    Diagnostic::at("main.mim", text, span, "boom")
}

#[test]
fn clean_cart() {
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/carts/hello"));
    assert_eq!(check(&Cart::from_dir(dir).unwrap()), []);
}

#[test]
fn parse_error() {
    let problem = one(&cart(
        r#"let pair = ("é😀" 1);
           luna::draw(|| {});"#,
    ));
    assert_eq!(problem.file, "main.mim");
    assert_eq!(span(&problem), (1, 18, 1, 18));
    assert_eq!(problem.message, "expected token");
    assert_eq!(problem.kind, DiagnosticKind::Error);
}

#[test]
fn type_error() {
    let problem = one(&cart(
        r#"gfx::text("é😀", std::math::ivec2(1, 2), 3);
           luna::draw(|| {});"#,
    ));
    assert_eq!(problem.file, "main.mim");
    assert_eq!(span(&problem), (1, 42, 1, 43));
    assert_eq!(problem.message, "mismatched types");
    assert_eq!(
        problem.label.as_deref(),
        Some("expected Color but found int")
    );
    assert_eq!(problem.kind, DiagnosticKind::Error);
}

#[test]
fn type_error_in_module() {
    let problem = one(&files(&[
        ("main.mim", "luna::draw(|| {});"),
        (
            "util.mim",
            r#"module @;

               pub fn text() {
                   gfx::text("é😀", std::math::ivec2(1, 2), 3);
               }"#,
        ),
    ]));
    assert_eq!(problem.file, "util.mim");
    assert_eq!(span(&problem), (4, 61, 4, 62));
    assert_eq!(problem.message, "mismatched types");
}

#[test]
fn runtime_fault() {
    let mut machine = Machine::load(
        &cart(
            r#"luna::update(|| print(("é😀", 1 ~/ luna::frame())));
               luna::draw(|| {});"#,
        ),
        1,
    )
    .unwrap();
    machine.frame(&Input::default());
    let problem = machine.fault().expect("the machine faulted");
    assert_eq!(problem.file, "main.mim");
    assert_eq!(span(problem), (1, 31, 1, 49));
    assert_eq!(problem.message, "divided by zero");
    assert_eq!(problem.label.as_deref(), Some("the divisor is zero"));
    assert_eq!(problem.kind, DiagnosticKind::Fault);
}

#[test]
fn runtime_fault_in_module() {
    let cart = files(&[
        ("main.mim", "luna::draw(|| util::half(luna::frame()));"),
        (
            "util.mim",
            r#"module @;

               pub fn half(n: int) {
                   print(("é😀", 1 ~/ n));
               }"#,
        ),
    ]);
    assert_eq!(check(&cart), []);
    let mut machine = Machine::load(&cart, 1).unwrap();
    machine.frame(&Input::default());
    let problem = machine.fault().expect("the machine faulted");
    assert_eq!(problem.file, "util.mim");
    assert_eq!(span(problem), (4, 34, 4, 40));
}

#[test]
fn load_fault() {
    let found = problems(
        r#"print(("é😀", 1 ~/ luna::frame()));
           luna::draw(|| {});"#,
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].file, "main.mim");
    assert_eq!(span(&found[0]), (1, 15, 1, 33));
    assert_eq!(found[0].kind, DiagnosticKind::Fault);
}

#[test]
fn load_error() {
    let main = r#"let a: int = "x";
                  luna::draw(|| {});"#;
    assert_eq!(problems(main), check(&cart(main)));
}

#[test]
fn every_parse_error() {
    let found = check(&cart(
        "let a = 1 +;
         let b = ;
         luna::draw(|| {});",
    ));
    assert_eq!(found.len(), 2);
    assert_eq!(span(&found[0]), (1, 12, 1, 13));
    assert_eq!(span(&found[1]), (2, 18, 2, 19));
    for problem in &found {
        assert_eq!(problem.message, "expected expression");
    }
}

#[test]
fn first_type_error() {
    let problem = one(&cart(
        r#"let a: int = "x";
           let b: int = "y";
           luna::draw(|| {});"#,
    ));
    assert_eq!(span(&problem), (1, 14, 1, 17));
}

#[test]
fn help_text() {
    let problem = one(&cart(
        r#"let s = "ab";
           let c = s["k"];
           luna::draw(|| {});"#,
    ));
    assert_eq!(problem.message, "strings are indexed by int");
    assert_eq!(problem.label.as_deref(), Some("this key isn't an int"));
    assert_eq!(
        problem.help.as_deref(),
        Some("index with a character position, like `s[0]`")
    );
}

#[test]
fn rules_and_errors() {
    let cart = files(&[
        (
            "main.mim",
            r#"let a: int = "x";
               luna::draw(|| {});"#,
        ),
        ("other.mim", "print(2);"),
    ]);
    let found: Vec<_> = check(&cart)
        .into_iter()
        .map(|problem| (problem.file, problem.message))
        .collect();
    let several = "a cart has several scripts: main.mim, other.mim";
    let expected = [
        ("main.mim", several),
        ("other.mim", several),
        ("main.mim", "mismatched types"),
    ]
    .map(|(file, message)| (file.to_owned(), message.to_owned()));
    assert_eq!(found, expected);
}

#[test]
fn shared_library() {
    let library = library();
    let broken = cart(
        r#"let a: int = "x";
           luna::draw(|| {});"#,
    );
    let fine = cart("luna::draw(|| gfx::clear(Color::Navy));");
    for _ in 0..2 {
        assert_eq!(check_with(&broken, &library), check(&broken));
        assert_eq!(check_with(&fine, &library), []);
    }
}

#[test]
fn at_lines_and_columns() {
    let problem = at("let x = 1;\nlet y = ;", 19..20);
    assert_eq!(problem.file, "main.mim");
    assert_eq!(problem.message, "boom");
    assert_eq!((problem.line, problem.col), (2, 9));
    assert_eq!((problem.end_line, problem.end_col), (2, 10));
    assert_eq!((problem.label, problem.help), (None, None));

    let problem = at("one\ntwo\nthree", 2..9);
    assert_eq!((problem.line, problem.col), (1, 3));
    assert_eq!((problem.end_line, problem.end_col), (3, 2));

    let problem = at("abc\n", 4..4);
    assert_eq!((problem.line, problem.col), (2, 1));
    let problem = at("", 0..0);
    assert_eq!(
        (problem.line, problem.col, problem.end_line, problem.end_col),
        (1, 1, 1, 1)
    );
}

#[test]
fn at_utf16_columns() {
    let problem = at("\u{e9}\u{1f600}x", 6..7);
    assert_eq!((problem.line, problem.col), (1, 4));
    assert_eq!((problem.end_line, problem.end_col), (1, 5));
    let problem = at("\u{1f600}\n\u{1f600}\u{1f600}x", 13..14);
    assert_eq!((problem.line, problem.col), (2, 5));
}

#[test]
fn at_stays_in_the_text() {
    let problem = at("\u{e9}", 1..99);
    assert_eq!((problem.line, problem.col), (1, 1));
    assert_eq!((problem.end_line, problem.end_col), (1, 2));
    let problem = at("ab", 50..60);
    assert_eq!((problem.col, problem.end_col), (3, 3));
}

#[test]
fn json_shape() {
    let mut problem = at("a", 0..1);
    problem.label = Some("here".into());
    let json = serde_json::to_value(&problem).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "file": "main.mim",
            "line": 1,
            "col": 1,
            "end_line": 1,
            "end_col": 2,
            "message": "boom",
            "label": "here",
            "help": null,
            "kind": "error",
        })
    );
    problem.kind = DiagnosticKind::Fault;
    let text = serde_json::to_string(&problem).unwrap();
    assert!(text.contains(r#""kind":"fault""#), "{text}");
    assert_eq!(serde_json::from_str::<Diagnostic>(&text).unwrap(), problem);
}
