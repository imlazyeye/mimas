use std::{cell::RefCell, path::Path, rc::Rc};

use lunacade_core::{Cart, Diagnostic, DiagnosticKind};
use mimas::{library, vm::Vm};

fn cart(files: &[(&str, &str)]) -> Cart {
    Cart {
        files: files
            .iter()
            .map(|(path, text)| (path.to_string(), text.to_string()))
            .collect(),
    }
}

fn with_sprites(text: &str) -> Cart {
    cart(&[("main.mim", "print(1);"), ("sprites.txt", text)])
}

fn problems(cart: &Cart) -> Vec<Diagnostic> {
    cart.validate().err().expect("the cart has problems")
}

fn run(cart: &Cart) -> Vec<String> {
    let valid = cart.validate().unwrap();
    let mut vm = Vm::compile_files(&valid.sources, library::sandboxed).unwrap();
    let lines = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::clone(&lines);
    vm.fixture::<library::Output>()
        .set(move |line| sink.borrow_mut().push(line.to_owned()));
    vm.run().unwrap();
    lines.take()
}

#[test]
fn sprites_parse() {
    let sheet = with_sprites("1a\n0F").validate().unwrap().sheet;
    assert_eq!(sheet[..2], [1, 10]);
    assert_eq!(sheet[128..130], [0, 15]);
    assert_eq!(sheet.iter().filter(|&&pixel| pixel != 0).count(), 3);

    let sheet = with_sprites("12\r\n34\r\n").validate().unwrap().sheet;
    assert_eq!(sheet[..2], [1, 2]);
    assert_eq!(sheet[128..130], [3, 4]);

    let line = "f".repeat(128);
    let text = vec![line.as_str(); 128].join("\n") + "\n\n\n";
    let sheet = with_sprites(&text).validate().unwrap().sheet;
    assert!(sheet.iter().all(|&pixel| pixel == 15));

    let cart = cart(&[("main.mim", "print(1);")]);
    let sheet = cart.validate().unwrap().sheet;
    assert!(sheet.iter().all(|&pixel| pixel == 0));
}

#[test]
fn sprites_bad_digit() {
    let problems = problems(&with_sprites("00\n0g\n"));
    assert_eq!(problems.len(), 1);
    let problem = &problems[0];
    assert_eq!(problem.file, "sprites.txt");
    assert_eq!((problem.line, problem.col), (2, 2));
    assert_eq!((problem.end_line, problem.end_col), (2, 3));
    assert!(problem.message.contains("`g`"), "{}", problem.message);
}

#[test]
fn sprites_too_big() {
    assert!(with_sprites(&"1".repeat(128)).validate().is_ok());
    let problem = &problems(&with_sprites(&"1".repeat(129)))[0];
    assert_eq!((problem.line, problem.col, problem.end_col), (1, 129, 130));

    assert!(with_sprites(&"0\n".repeat(128)).validate().is_ok());
    let problem = &problems(&with_sprites(&"0\n".repeat(129)))[0];
    assert_eq!((problem.line, problem.col), (129, 1));
    assert_eq!((problem.end_line, problem.end_col), (129, 2));
}

#[test]
fn one_script() {
    let cart = cart(&[("main.mim", "print(1);"), ("other.mim", "print(2);")]);
    let problems = problems(&cart);
    let files: Vec<_> = problems
        .iter()
        .map(|problem| problem.file.as_str())
        .collect();
    assert_eq!(files, ["main.mim", "other.mim"]);
    for problem in &problems {
        let message = "a cart has several scripts: main.mim, other.mim";
        assert_eq!(problem.message, message);
        assert_eq!(problem.kind, DiagnosticKind::Error);
    }
}

#[test]
fn script_by_any_name() {
    let cart = cart(&[("pong.mim", "print(1);"), ("ball.mim", "module @;")]);
    assert_eq!(cart.validate().unwrap().script, "pong.mim");
    assert_eq!(run(&cart), ["1"]);
}

#[test]
fn empty_file_is_a_script() {
    let cart = cart(&[("main.mim", "print(1);"), ("empty.mim", "")]);
    assert_eq!(problems(&cart)[0].file, "empty.mim");
}

#[test]
fn no_script() {
    for cart in [cart(&[]), cart(&[("main.mim", "module @;")])] {
        let problems = problems(&cart);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].file, "");
        assert_eq!(problems[0].message, "a cart has no script to run");
        assert_eq!((problems[0].line, problems[0].col), (1, 1));
    }
}

#[test]
fn every_problem_at_once() {
    let cart = cart(&[
        ("main.mim", "print(1);"),
        ("other.mim", "print(2);"),
        ("sprites.txt", "xyz"),
    ]);
    let problems = problems(&cart);
    let files: Vec<_> = problems
        .iter()
        .map(|problem| problem.file.as_str())
        .collect();
    assert_eq!(files, ["main.mim", "other.mim", "sprites.txt"]);
}

#[test]
fn source_order() {
    let module = "module @;";
    let cart = cart(&[
        ("z.mim", module),
        ("game.mim", "print(1);"),
        ("b.mim", module),
        ("a.mim", module),
    ]);
    let valid = cart.validate().unwrap();
    let paths: Vec<_> = valid.sources.iter().map(|(path, _)| *path).collect();
    assert_eq!(paths, ["a.mim", "b.mim", "z.mim", "game.mim"]);
}

#[test]
fn modules_are_visible() {
    let cart = cart(&[
        (
            "main.mim",
            "use util;
             print(util::double(21));
             print(shapes::SIDES);",
        ),
        (
            "util.mim",
            "module @;
             pub fn double(n: int) -> int {
                 n * 2
             }",
        ),
        (
            "shapes.mim",
            "module @;
             pub const SIDES = 4;",
        ),
    ]);
    assert_eq!(run(&cart), ["42", "4"]);
}

#[test]
fn from_dir_hello() {
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/carts/hello"));
    let cart = Cart::from_dir(dir).unwrap();
    let paths: Vec<_> = cart.files.keys().map(String::as_str).collect();
    assert_eq!(paths, ["hello.mim", "sprites.txt"]);
    let valid = cart.validate().unwrap();
    assert_eq!(valid.sheet[2], 4);
    assert_eq!(valid.sheet[128 + 1], 4);
}

#[test]
fn from_dir_parts_only() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("from_dir_parts_only");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    for name in [
        "main.mim",
        "util.mim",
        "sprites.txt",
        "notes.txt",
        "cart.toml",
        "sub/inner.mim",
    ] {
        std::fs::write(dir.join(name), name).unwrap();
    }
    let cart = Cart::from_dir(&dir).unwrap();
    let paths: Vec<_> = cart.files.keys().map(String::as_str).collect();
    assert_eq!(paths, ["main.mim", "sprites.txt", "util.mim"]);
    assert_eq!(cart.files["util.mim"], "util.mim");
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(Cart::from_dir(&dir).is_err());
}

#[test]
fn json_shape() {
    let json = r#"{"id":"pong","files":{"main.mim":"print(1);","sprites.txt":"1"}}"#;
    let cart: Cart = serde_json::from_str(json).unwrap();
    assert_eq!(cart.files.len(), 2);
    assert_eq!(cart.files["main.mim"], "print(1);");
    assert!(
        serde_json::to_string(&cart)
            .unwrap()
            .starts_with(r#"{"files":{"main.mim""#)
    );
}
