#![allow(dead_code, unused_macros)]

use std::cell::RefCell;

use crate::{components::*, *};
use parse::{Ident, Parser, lex::Lexer};
use pretty_assertions::assert_eq;

thread_local! {
    pub static TEST_SESSION: RefCell<TestSession> =
        RefCell::new(TestSession(Solver::new()));
}

pub(crate) struct TestSession(pub Solver);
impl TestSession {
    pub(crate) fn reset(&mut self) {
        self.0 = Solver::new();
    }

    pub(crate) fn run(&mut self, source: &str, file_name: impl Into<String>) -> Result<()> {
        let file_name = file_name.into();
        let source = Box::leak(Box::new(source.to_string()));
        let mut sources = std::collections::HashMap::new();
        sources.insert(
            0,
            miette::NamedSource::new(&file_name, std::sync::Arc::<str>::from(source.as_str())),
        );
        self.0.set_sources(sources);
        let lexer = Lexer::new(source, 0, file_name);
        let parser = Parser::new(lexer);
        let ast = parser.into_ast().unwrap();
        self.0.solve(&ast)?;
        Ok(())
    }

    pub(crate) fn test_ty(
        &mut self,
        should_be: Ty,
        preamble: Option<&str>,
        src: &'static str,
        modules: &[&str],
    ) {
        let uses: String = modules.iter().map(|m| format!("use {}; ", m)).collect();
        let source = Box::leak(Box::new(format!("{uses}let TEST_VALUE = {src};")));
        self.0.ribs.push_block();
        self.run(source, "test").unwrap();
        let ty = self
            .0
            .resolve_name(&Ident::synthetic("TEST_VALUE"), shared::Location::default())
            .unwrap();
        self.0.ribs.pop();
        if !should_be.loose_eq(&ty, &self.0) {
            let lhs = crate::utils::Printer::ty(&should_be);
            let rhs = crate::utils::Printer::ty(&ty);
            if let Some(preamble) = preamble {
                println!("\n-- Preamble --\n{preamble}");
            }
            println!("\n-- Source -- \n{source}");

            assert_eq!(lhs, rhs);
            panic!("strings were equal but we failed!");
        }
    }
}

pub(crate) struct TestResetter;
impl Drop for TestResetter {
    fn drop(&mut self) {
        TEST_SESSION.with(|s| s.borrow_mut().reset());
    }
}

pub trait LooseEq {
    fn loose_eq(&self, other: &Self, solver: &Solver) -> bool;
}

impl LooseEq for Ty {
    fn loose_eq(&self, other: &Ty, solver: &Solver) -> bool {
        match (self, other) {
            (Ty::Vid(_), Ty::Vid(_)) => true, // may live to regret that?
            (Ty::Array(ty), Ty::Array(o_ty)) | (Ty::Dict(ty), Ty::Dict(o_ty)) => {
                ty.loose_eq(o_ty, solver)
            }
            (Ty::Adt(adt), Ty::Adt(o_adt))
            | (Ty::Identity(adt), Ty::Adt(o_adt))
            | (Ty::Adt(adt), Ty::Identity(o_adt)) => {
                adt == o_adt || (solver.adts[adt].loose_eq(&solver.adts[o_adt], solver))
            }

            (Ty::Tuple(l), Ty::Tuple(r)) => {
                l.len() == r.len() && l.iter().zip(r.iter()).all(|(a, b)| a.loose_eq(b, solver))
            }
            (Ty::Fn(l), Ty::Fn(r)) => l.loose_eq(r, solver),
            (Ty::Option(l), Ty::Option(r)) => l.loose_eq(r, solver),
            _ => self == other,
        }
    }
}

impl LooseEq for Adt {
    fn loose_eq(&self, other: &Self, solver: &Solver) -> bool {
        self.variants.len() == other.variants.len()
            && self.variants.iter().all(|(name, variant)| {
                other
                    .variants
                    .get(name)
                    .is_some_and(|o| variant.loose_eq(o, solver))
            })
    }
}

impl LooseEq for Variant {
    fn loose_eq(&self, other: &Self, solver: &Solver) -> bool {
        println!("{} {}", self, other);
        match (self, other) {
            (Variant::Tuple(variant), Variant::Tuple(o_variant)) => variant
                .members
                .iter()
                .zip(o_variant.members.iter())
                .all(|(a, b)| a.loose_eq(b, solver)),
            (Variant::Struct(variant), Variant::Struct(o_variant)) => {
                variant.fields.len() == o_variant.fields.len()
                    && variant.fields.iter().all(|(name, field)| {
                        o_variant
                            .fields
                            .get(name)
                            .is_some_and(|o| field.loose_eq(o, solver))
                    })
            }
            _ => false,
        }
    }
}

impl LooseEq for Field {
    fn loose_eq(&self, other: &Self, solver: &Solver) -> bool {
        self.ty.loose_eq(&other.ty, solver)
    }
}

impl LooseEq for FnHeader {
    fn loose_eq(&self, other: &Self, solver: &Solver) -> bool {
        self.parameters
            .iter()
            .zip(other.parameters.iter())
            .all(|(l, r)| l.ty.loose_eq(&r.ty, solver))
            && self.return_ty.loose_eq(&other.return_ty, solver)
    }
}

macro_rules! array {
    ($ty:expr) => {{ $crate::components::Ty::Array(Box::new($ty)) }};
}

macro_rules! tuple {
    ($($ty:expr),*) => {{
        $crate::components::Ty::Tuple(vec![$($ty, )*])
    }};
}

macro_rules! dictionary {
    ($ty:expr) => {{ $crate::components::Ty::Dict(Box::new($ty)) }};
}

macro_rules! option {
    ($ty:expr) => {{ $crate::components::Ty::Option(Box::new($ty)) }};
}

macro_rules! result {
    ($ty:expr) => {{ $crate::components::Ty::Result(Box::new($ty)) }};
}

macro_rules! query {
    ($name:ident) => {{
        super::solve_test_utils::TEST_SESSION.with(|s| {
            s.borrow_mut()
                .0
                .resolve_name(
                    &parse::Ident::synthetic(stringify!($name)),
                    shared::Location::default(),
                )
                .unwrap()
        })
    }};
}

macro_rules! adt {
    ({}) => {{
        let adt = $crate::components::Adt::new_struct("TEST_STRUCT".into());
        let id = super::solve_test_utils::TEST_SESSION
            .with(|s| s.borrow_mut().0.adts.push(adt));
        $crate::components::Ty::Adt(id)
    }};

    ({ $($field: ident: $ty: expr),* }) => {{
        let mut adt = $crate::components::Adt::new_struct("TEST_STRUCT".into());
        $(
            let ty = $ty;
            let name: String = stringify!($field).into();
            let dec = super::solve_test_utils::TEST_SESSION.with(|s| {
                s.borrow_mut().0.dec_id(
                    &parse::Ident::synthetic(name.clone()),
                    ty.clone(),
                    $crate::components::DecKind::Local,
                    $crate::components::Vis::Public,
                )
            });
            let field = $crate::components::Field {
                ty: ty.clone(),
                declaration_location: shared::Location::default(),
                constant: false,
                dec,
            };
            if matches!(ty, $crate::components::Ty::Fn(_)) {
                adt.impls.insert(name, field);
            } else {
                adt.as_struct_mut().fields.insert(name, field);
            }
        )*
        let id = super::solve_test_utils::TEST_SESSION
            .with(|s| s.borrow_mut().0.adts.push(adt));
        $crate::components::Ty::Adt(id)
    }};
}

macro_rules! enu {
     ({}) => {{
        let adt = $crate::components::Adt::new_enum("TEST_ENUM".into(), Default::default());
        let id = super::solve_test_utils::TEST_SESSION
            .with(|s| s.borrow_mut().0.adts.push(adt));
        $crate::components::Ty::Adt(id)
    }};
    ({ $($variant:expr),*$(,)? }) => {{
        let variants = vec![
            $($variant),*
        ].into_iter().collect();
        let adt = $crate::components::Adt::new_enum("TEST_ENUM".into(), variants);
        let id = super::solve_test_utils::TEST_SESSION
            .with(|s| s.borrow_mut().0.adts.push(adt));
        $crate::components::Ty::Adt(id)
    }};
}

macro_rules! enum_member {
    ($name:ident) => {{
        (
            stringify!($name).to_string(),
            $crate::components::Variant::Struct($crate::components::StructVariant::default()),
        )
    }};

    ($name:ident { $($field:ident: $ty: expr),*$(,)? }) => {{
        let fields = vec![$({
            let field_name: String = stringify!($field).to_string();
            let ty = $ty;
            let dec = super::solve_test_utils::TEST_SESSION.with(|s| {
                s.borrow_mut().0.dec_id(
                    &parse::Ident::synthetic(field_name.clone()),
                    ty.clone(),
                    $crate::components::DecKind::Local,
                    $crate::components::Vis::Public,
                )
            });
            (
                field_name,
                $crate::components::Field {
                    ty,
                    declaration_location: shared::Location::default(),
                    constant: false,
                    dec,
                },
            )
        }),*];

        (
            stringify!($name).to_string(),
            $crate::components::Variant::Struct($crate::components::StructVariant {
                fields: $crate::components::Fields(fields.into_iter().collect()),
                ..Default::default()
            }),
        )
    }};

    ($name:ident ($($ty:expr),*$(,)?)) => {{
        (
            stringify!($name).to_string(),
            $crate::components::Variant::Tuple($crate::components::TupleVariant {
                members: vec![$($ty, )*],
                ..Default::default()
            }),
        )
    }}
}

macro_rules! func {
    (() -> $return_ty:expr) => {{
        $crate::components::Ty::Fn($crate::components::FnHeader::new(vec![], $return_ty, false))
    }};
    (($($arg:expr), * $(,)?) -> $return_ty:expr) => {{
        $crate::components::Ty::Fn($crate::components::FnHeader::new(
            vec![$($crate::components::FnParam::new(None, $arg, false), )*], $return_ty, false))
    }};
}

macro_rules! unknown {
    () => {
        $crate::components::Ty::Vid($crate::components::Vid::UNKNOWN)
    };
}

#[macro_export]
macro_rules! test_ty {
    ($(#[$attr:meta])* $name:ident, $src:expr $(,)?) => {
        #[cfg(test)]
        #[test]
        $(#[$attr])*
        fn $name() {
            use super::solve_test_utils::*;
            let _t = TestResetter;
            TEST_SESSION.with(|s| s.borrow_mut().run($src, "test")).unwrap();
        }
    };
    ($(#[$attr:meta])* $name:ident, $($src:expr => $should_be:expr), * $(,)?) => {
        #[cfg(test)]
        #[test]
        $(#[$attr])*
        fn $name() {
            use super::solve_test_utils::*;
            let _t = TestResetter;
            $({
                let should_be = $should_be;
                TEST_SESSION.with(|s| s.borrow_mut().test_ty(should_be, None, $src, &[]));
            })*
        }
    };
    ($(#[$attr:meta])* $name:ident, $preamble:expr, $($src:expr => $should_be:expr), * $(,)?) => {
        #[cfg(test)]
        #[test]
        $(#[$attr])*
        fn $name() {
            use super::solve_test_utils::*;
            let _t = TestResetter;
            TEST_SESSION.with(|s| s.borrow_mut().run($preamble, "test")).unwrap();
            $({
                let should_be = $should_be;
                TEST_SESSION.with(|s| s.borrow_mut().test_ty(should_be, None, $src, &[]));
            })*
        }
    };
}

#[macro_export]
macro_rules! test_multi_file {
    ($(#[$attr:meta])* $name:ident,
     $($file_name:ident => $file_src:expr),+ $(,)?
     ;
     $($test_src:literal => $should_be:expr),* $(,)?
    ) => {
        #[cfg(test)]
        #[test]
        $(#[$attr])*
        fn $name() {
            use super::solve_test_utils::*;
            let _t = TestResetter;
            let modules: &[&str] = &[$(stringify!($file_name)),+];
            let mut asts = Vec::new();
            $(
                let source = Box::leak(Box::new($file_src.to_string()));
                let lexer = parse::lex::Lexer::new(source, 0, stringify!($file_name).into());
                let parser = parse::Parser::new(lexer);
                asts.push(parser.into_ast().unwrap());
            )+
            TEST_SESSION
                .with(|s| s.borrow_mut().0.solve_all(asts.iter()))
                .unwrap();
            $({
                let should_be = $should_be;
                TEST_SESSION.with(|s| s.borrow_mut().test_ty(should_be, None, $test_src, modules));
            })*
        }
    };
}

/// Asserts that a multi-file program fails to type-check.
#[macro_export]
macro_rules! test_multi_file_fail {
    ($(#[$attr:meta])* $name:ident,
     $($file_name:ident => $file_src:expr),+ $(,)?
     ;
    ) => {
        #[cfg(test)]
        #[test]
        $(#[$attr])*
        fn $name() {
            use super::solve_test_utils::*;
            let _t = TestResetter;
            let mut asts = Vec::new();
            $(
                let source = Box::leak(Box::new($file_src.to_string()));
                let lexer = parse::lex::Lexer::new(source, 0, stringify!($file_name).into());
                let parser = parse::Parser::new(lexer);
                asts.push(parser.into_ast().unwrap());
            )+
            let result = TEST_SESSION.with(|s| s.borrow_mut().0.solve_all(asts.iter()));
            assert!(result.is_err(), "expected multi-file program to fail solve");
        }
    };
}

#[macro_export]
macro_rules! test_reduction {
    ($name:ident, $src:expr => None) => {
        #[cfg(test)]
        #[test]
        fn $name() {
            use super::solve_test_utils::*;
            use $crate::utils::reduce_simple;
            let _t = TestResetter;
            let lexer = parse::lex::Lexer::new($src, 0, "test".into());
            let mut parser = parse::Parser::new(lexer);
            let outputed = parser.expr().unwrap();
            let got = reduce_simple(&outputed).unwrap();
            pretty_assertions::assert_eq!(got, None);
        }
    };

    ($name:ident, $($src:expr => $should_be:expr), * $(,)?) => {
        #[cfg(test)]
        #[test]
        fn $name() {
            use super::solve_test_utils::*;
            use $crate::utils::reduce_simple;
            let _t = TestResetter;
            $({
                let lexer = parse::lex::Lexer::new($src, 0, "test".into());
                let mut parser = parse::Parser::new(lexer);
                let outputed = parser.expr().unwrap();
                let got = reduce_simple(&outputed).unwrap();
                if got != Some($should_be) {
                    println!("Failed on `{}`", $src);
                    pretty_assertions::assert_eq!(got, Some($should_be));
                }
            })*
        }
    };
}

#[macro_export]
macro_rules! test_success {
    ($name:ident, $src:expr) => {
        #[cfg(test)]
        #[test]
        fn $name() {
            use super::solve_test_utils::*;
            let _t = TestResetter;
            TEST_SESSION
                .with(|s| s.borrow_mut().run($src, "test"))
                .unwrap();
        }
    };
}

#[macro_export]
macro_rules! test_fail {
    ($(#[$attr:meta])* $name:ident, $($src:expr),+ $(,)?) => {
        #[cfg(test)]
        #[test]
        $(#[$attr])*
        fn $name() {
            use super::solve_test_utils::*;
            $({
                let _t = TestResetter;
                if TEST_SESSION.with(|s| s.borrow_mut().run($src, "test")).is_ok() {
                    panic!("expected solve to reject:\n{}", $src);
                }
            })+
        }
    };
}
