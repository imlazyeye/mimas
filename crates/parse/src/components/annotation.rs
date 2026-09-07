use crate::lex::TyKw;
use itertools::Itertools;

use crate::expr::Ident;

#[derive(Debug, PartialEq, Clone)]
pub enum Annotation {
    Unit,
    Kw(TyKw),
    Option(Box<Annotation>),
    Result(Box<Annotation>),
    Tuple(Vec<Annotation>),
    Array(Box<Annotation>),
    Dictionary(Box<Annotation>),
    Function(Vec<Annotation>, Box<Annotation>),
    Ty(Ident),
    Path(Vec<Ident>),
    Bounds(Vec<Ident>),
}

#[mutants::skip]
impl std::fmt::Display for Annotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Annotation::Unit => f.pad("()"),
            Annotation::Kw(tykw) => f.pad(&tykw.to_string()),
            Annotation::Option(t) => f.pad(&format!("{t}?")),
            Annotation::Result(t) => f.pad(&format!("{t}!")),
            Annotation::Tuple(members) => f.pad(&format!(
                "({})",
                members.iter().map(|p| p.to_string()).join(", "),
            )),
            Annotation::Array(e) => f.pad(&format!("[{e}]")),
            Annotation::Dictionary(e) => f.pad(&format!("~{{{e}}}")),
            Annotation::Function(params, ret) => f.pad(&format!(
                "({}) -> {ret}",
                params.iter().map(|p| p.to_string()).join(", "),
            )),
            Annotation::Ty(ident) => f.pad(&format!("{ident}")),
            Annotation::Path(idents) => f.pad(&idents.iter().map(|i| i.to_string()).join("::")),
            Annotation::Bounds(idents) => f.pad(&idents.iter().map(|i| i.to_string()).join(" + ")),
        }
    }
}
