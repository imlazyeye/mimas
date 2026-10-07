use vm::Vm;

const HOST_ONLY: [&str; 6] = [
    "std::fs::cwd();",
    "use std::fs;",
    r#"std::process::run("true", []);"#,
    "use std::process;",
    "std::sys::arg(0);",
    "use std::sys::exit;",
];

fn render(src: &str) -> String {
    let source = format!("let TEST_VALUE = {src};");
    let mut vm = Vm::execute(&source, library::sandboxed).expect("sandboxed script ran");
    format!(
        "{}",
        vm.resolve_name("TEST_VALUE").expect("TEST_VALUE was bound")
    )
}

#[test]
fn host_modules_missing() {
    for src in HOST_ONLY {
        assert!(
            Vm::compile(src, library::std).is_ok(),
            "std should resolve `{src}`"
        );
        assert!(
            Vm::compile(src, library::sandboxed).is_err(),
            "sandboxed resolved `{src}`"
        );
    }
}

#[test]
fn std_range_covers_install() {
    let library = Vm::new().install_library(library::sandboxed).without_std();
    assert_eq!(library.natives().count(), 0);
    assert!(library.adts().is_empty());
}

#[test]
fn prelude_works() {
    assert_eq!(render(r#"{ print("hi"); 0 }"#), "0");
    assert_eq!(render(r#"f"{1 + 2} apples""#), r#""3 apples""#);
}

#[test]
fn math_works() {
    assert_eq!(render("std::math::Vec2::new(3.0, 4.0).length()"), "5");
    assert_eq!(render("std::math::PI > 3.0"), "true");
}

#[test]
fn parse_works() {
    assert_eq!(
        render(r#"std::parse::to_json(std::parse::from_json("[1, 2]")!)!"#),
        r#""[1,2]""#,
    );
}

#[test]
fn methods_work() {
    assert_eq!(render(r#""a,b".split(",").len()"#), "2");
    assert_eq!(render("[1, 2, 3].reversed()"), "[3, 2, 1]");
    assert_eq!(render("(-4).abs()"), "4");
    assert_eq!(render("(~{ a = 1 }).len()"), "1");
}
