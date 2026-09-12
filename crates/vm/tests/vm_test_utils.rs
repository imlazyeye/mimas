#![allow(dead_code, unused_macros, unused_imports)]

pub use vm::Captured;
use vm::Vm;

#[track_caller]
pub fn assert_output(source: &str, expected: Captured) {
    let mut vm = Vm::execute(source, |_| {}).expect("test source compiled and ran");
    let actual = vm
        .resolve_name("TEST_VALUE")
        .expect("TEST_VALUE was bound by the test source");
    pretty_assertions::assert_eq!(actual, expected, "failed on source:\n{source}");
}

// multi-file variant: `files` is `(stem, source)` pairs; one must be named `main` and
// must bind `TEST_VALUE`. others are pulled in as modules under their stem.
#[track_caller]
pub fn assert_files(files: &[(&str, &str)], expected: Captured) {
    let mut vm = Vm::execute_files(files, |_| {}).expect("test files compiled and ran");
    let actual = vm
        .resolve_name("TEST_VALUE")
        .expect("TEST_VALUE was bound by the main file");
    pretty_assertions::assert_eq!(actual, expected, "failed on files: {files:?}");
}

#[track_caller]
pub fn assert_fails(source: &str) {
    let result = Vm::execute(source, |_| {});
    assert!(result.is_err(), "expected source to be rejected:\n{source}");
}

#[macro_export]
macro_rules! str {
    ($s:expr) => {{ $crate::vm_test_utils::Captured::Str(($s).to_string()) }};
}

// `array!(Int(0), str!("foo"))` -> `Captured::Array(vec![..])`
#[macro_export]
macro_rules! array {
    ( $($val:expr),* $(,)? ) => {{
        $crate::vm_test_utils::Captured::Array(vec![ $($val),* ])
    }};
}

// mirrors mimas's `~{}` dict literal so the expected reads like the source
#[macro_export]
macro_rules! dict {
    ( ~{ $($key:ident = $val:expr),* $(,)? } ) => {{
        $crate::vm_test_utils::Captured::Dict(::std::vec![
            $( (stringify!($key).to_string(), $val) ),*
        ])
    }};
}

// the struct name is accepted for readability but ignored at comparison time -- `Captured`
// compares instance fields positionally, with no name involved. The runtime does now carry
// struct/variant names (used by `print`/`display`/f-strings, see types.rs's display tests);
// this macro just never needed them for equality.
#[macro_export]
macro_rules! instance {
    ( $name:ident { $($field:ident = $val:expr),* $(,)? } ) => {{
        let _ = stringify!($name);
        $crate::vm_test_utils::Captured::Instance(vec![ $($val),* ])
    }};
}

// the test runner. three call shapes:
//   1. bare evals:      test_vm!(name, "expr" => Expected, "expr2" => Expected2, ...)
//   2. preamble+evals:  test_vm!(name, "let a = 0;", "a" => Int(0), ...)
//   3. multi-file:      test_vm!(name, files { foo => "...", main => "..." } => Expected)
// attribute pass-through: test_vm!(#[ignore = "wip"] name, ...)
#[macro_export]
macro_rules! test_vm {
    // multi-file form -- must come first (the `files { ... }` block would otherwise be
    // parsed as a preamble expression by the next arm).
    (
        $(#[$attr:meta])* $name:ident,
        files { $($file:ident => $src:expr),* $(,)? } => $expected:expr $(,)?
    ) => {
        #[test]
        $(#[$attr])*
        fn $name() {
            let files: &[(&str, &str)] = &[ $((stringify!($file), $src)),* ];
            $crate::vm_test_utils::assert_files(files, $expected);
        }
    };

    // preamble form -- preamble is any non-eval expression, followed by `, "src" => Expected, ...`.
    (
        $(#[$attr:meta])* $name:ident,
        $preamble:expr, $($src:expr => $expected:expr),* $(,)?
    ) => {
        #[test]
        $(#[$attr])*
        fn $name() {
            $(
                let __source = format!("{}\nlet TEST_VALUE = {};", $preamble, $src);
                $crate::vm_test_utils::assert_output(&__source, $expected);
            )*
        }
    };

    // bare-evals form -- no preamble, just `"src" => Expected, ...`.
    (
        $(#[$attr:meta])* $name:ident,
        $($src:expr => $expected:expr),* $(,)?
    ) => {
        #[test]
        $(#[$attr])*
        fn $name() {
            $(
                let __source = format!("let TEST_VALUE = {};", $src);
                $crate::vm_test_utils::assert_output(&__source, $expected);
            )*
        }
    };
}

// each source must be rejected by the pipeline (compile or run). multi-case:
// test_fail!(name, "a", "b", "c") -> one #[test] asserting each fails.
#[macro_export]
macro_rules! test_fail {
    ( $(#[$attr:meta])* $name:ident, $($src:expr),+ $(,)? ) => {
        #[test]
        $(#[$attr])*
        fn $name() {
            $( $crate::vm_test_utils::assert_fails($src); )+
        }
    };
}
