use itertools::Itertools;

use crate::{
    expr::Ident,
    item::{IntoItem, ItemKind},
};

#[derive(Debug, PartialEq, Clone)]
pub enum Use {
    Singular(Vec<Ident>, Ident),
    Multi(Vec<Ident>, Vec<Ident>),
    All(Vec<Ident>),
}
impl From<Use> for ItemKind {
    fn from(us: Use) -> Self {
        Self::Use(us)
    }
}
impl IntoItem for Use {}

#[mutants::skip]
impl std::fmt::Display for Use {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Use::Singular(path, item) => {
                f.pad(&format!("use {}::{};", path.iter().join("::"), item))
            }
            Use::Multi(path, items) => f.pad(&format!(
                "use {}::{{ {} }};",
                path.iter().join("::"),
                items.iter().join(", ")
            )),
            Use::All(path) => f.pad(&format!("use {}::*;", path.iter().join("::"))),
        }
    }
}
