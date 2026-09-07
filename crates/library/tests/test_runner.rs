#![allow(dead_code)]

use vm::Vm;

pub fn render(preamble: &str, src: &str) -> String {
    let source = format!("{preamble}\nlet TEST_VALUE = {src};");
    let mut vm = Vm::execute(&source, library::std).expect("library test compiled");
    let captured = vm.resolve_name("TEST_VALUE").expect("TEST_VALUE was bound");
    format!("{captured}")
}

pub fn try_execute(src: &str) -> Result<(), vm::ExecuteError> {
    Vm::execute(src, library::std).map(|_| ())
}

#[macro_export]
macro_rules! test_fail {
    ($(#[$attr:meta])* $name:ident, $($src:expr),+ $(,)?) => {
        #[cfg(test)]
        #[test]
        $(#[$attr])*
        fn $name() {
            $(
                if $crate::test_runner::try_execute($src).is_ok() {
                    panic!("expected compilation/runtime failure for `{}`", $src);
                }
            )+
        }
    };
}

#[macro_export]
macro_rules! test_run {
    ($(#[$attr:meta])* $name:ident, $($src:expr => $expected:expr),* $(,)?) => {
        #[cfg(test)]
        #[test]
        $(#[$attr])*
        fn $name() {
            $({
                let actual = $crate::test_runner::render("", $src);
                pretty_assertions::assert_eq!(actual, $expected, "failed on `{}`", $src);
            })*
        }
    };
    ($(#[$attr:meta])* $name:ident, $preamble:expr, $($src:expr => $expected:expr),* $(,)?) => {
        #[cfg(test)]
        #[test]
        $(#[$attr])*
        fn $name() {
            $({
                let actual = $crate::test_runner::render($preamble, $src);
                pretty_assertions::assert_eq!(actual, $expected, "failed on `{}`", $src);
            })*
        }
    };
}
