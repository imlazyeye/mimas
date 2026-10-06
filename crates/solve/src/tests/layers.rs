use api::Library;
use parse::Ident;
use shared::Location;

use crate::{Modules, components::Ty};

fn empty() -> Modules {
    Modules::from_files(std::iter::empty::<(&str, &str)>(), &Library::new())
}

fn with_module(name: &str, text: &str) -> Modules {
    Modules::from_files([(name, text)], &Library::new())
}

fn layer(base: &Modules, text: &str) -> Modules {
    base.load([("<repl>", text)])
}

fn ty(modules: &Modules, name: &str) -> Ty {
    modules
        .solver
        .clone()
        .resolve_name(&Ident::synthetic(name), Location::default())
        .unwrap()
}

fn clean(modules: &Modules) {
    assert!(modules.errors.is_empty(), "{:?}", modules.errors);
}

#[test]
fn layers_see_the_modules() {
    let base = Modules::from_files(
        [
            (
                "p/m.mim",
                "module @;
                 pub fn one() -> int { 1 }",
            ),
            (
                "p/sub/n.mim",
                "module @;
                 pub fn two() -> int { 2 }",
            ),
        ],
        &Library::new(),
    );
    clean(&base);
    let script = layer(
        &base,
        "use m;
         use n;
         let x: int = m::one() + n::two();",
    );
    clean(&script);
}

#[test]
fn layers_are_isolated() {
    let base = empty();
    clean(&layer(&base, "fn shared() -> int { 1 }"));
    assert!(!layer(&base, "let x: int = shared();").errors.is_empty());
}

#[test]
fn broken_modules_cut_off_the_layer() {
    let base = with_module(
        "m.mim",
        r#"module @;
           pub fn f() -> int { "no" }"#,
    );
    assert_eq!(base.errors.len(), 1);
    assert!(layer(&base, "let x: str = 1;").errors.is_empty());
}

#[test]
fn let_type_is_fixed() {
    let first = layer(&empty(), "let a = 1;");
    let second = layer(&first, r#"a = "x";"#);
    assert!(!second.errors.is_empty());
}

#[test]
fn fn_sees_no_lets() {
    let first = layer(&empty(), "let a = 1;");
    let second = layer(&first, "fn f() -> int { a }");
    assert!(!second.errors.is_empty());
}

#[test]
fn items_carry() {
    let first = layer(
        &empty(),
        "fn f() -> int { 1 }
         struct P { x: int }
         enum E { A, B }
         const C: int = 3;",
    );
    let second = layer(
        &first,
        "let v = f() + P { x = 1 }.x + C;
         let e = E::A;",
    );
    clean(&second);
    assert_eq!(ty(&second, "v"), Ty::Int);
}

#[test]
fn use_item_carries() {
    let base = with_module(
        "m.mim",
        "module @;
         pub fn one() -> int { 1 }",
    );
    clean(&base);
    let first = layer(&base, "use m::one;");
    clean(&first);
    let second = layer(&first, "let x = one();");
    clean(&second);
}

#[test]
fn enum_redefinition_wins() {
    let first = layer(&empty(), "enum E { A }");
    let second = layer(&first, "enum E { B }");
    let third = layer(&second, "let e = E::B;");
    clean(&third);
    let bad = layer(&second, "let bad = E::A;");
    assert!(!bad.errors.is_empty());
}

#[test]
fn pact_redefinition_wins() {
    let first = layer(
        &empty(),
        r#"pact Greet {
               fn hi(self) -> str;
           }
           struct A {}
           impl Greet for A {
               fn hi(self) -> str {
                   "a"
               }
           }"#,
    );
    clean(&first);
    let second = layer(
        &first,
        "pact Greet {
             fn hi(self) -> int;
         }",
    );
    clean(&second);
    let third = layer(
        &second,
        "struct B {}
         impl Greet for B {
             fn hi(self) -> int {
                 1
             }
         }",
    );
    clean(&third);
}

#[test]
fn const_redefinition_rejected() {
    let first = layer(&empty(), "const C: int = 1;");
    let second = layer(&first, "const C: int = 2;");
    assert!(!second.errors.is_empty());
}

#[test]
fn method_redefinition_rejected() {
    let first = layer(
        &empty(),
        "struct P {}
         impl P {
             fn m(self) -> int {
                 1
             }
         }",
    );
    let second = layer(
        &first,
        "impl P {
             fn m(self) -> int {
                 2
             }
         }",
    );
    assert!(!second.errors.is_empty());
}
