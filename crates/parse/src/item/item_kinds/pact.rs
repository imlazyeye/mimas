use crate::{
    Expr,
    components::{Annotation, Binding},
    expr::Ident,
    item::{IntoItem, ItemKind},
};
use itertools::Itertools;

/// Representation of a `pact` declaration in mimas.
#[derive(Debug, PartialEq, Clone)]
pub struct Pact {
    pub name: Ident,
    pub items: Vec<PactItem>,
}
impl Pact {
    pub(crate) fn new(name: Ident, items: Vec<PactItem>) -> Self {
        Self { name, items }
    }
}

impl From<Pact> for ItemKind {
    fn from(p: Pact) -> Self {
        Self::Pact(p)
    }
}
impl IntoItem for Pact {}

#[mutants::skip]
impl std::fmt::Display for Pact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.items.is_empty() {
            f.pad(&format!("pact {} {{}}", self.name))
        } else {
            f.pad(&format!(
                "pact {} {{ {} }}",
                self.name,
                self.items.iter().join(" ")
            ))
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum PactItem {
    /// A constant signature: `const NAME: T;`. Pacts cannot declare default values for constants
    /// (an `impl` block must always supply one).
    Const { name: Ident, annotation: Annotation },
    /// A method signature: `fn name(params) -> ret;` or, with a default body, `fn name(params) ->
    /// ret { body }`. Parameters may not have default values in a pact signature.
    Fn {
        name: Ident,
        parameters: Vec<Binding>,
        return_type: Option<Annotation>,
        /// `Some(body)` if the pact provides a default implementation. Impls may omit this method
        /// when a default exists.
        default: Option<Expr>,
    },
}

#[mutants::skip]
impl std::fmt::Display for PactItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PactItem::Const { name, annotation } => {
                f.pad(&format!("const {}: {};", name, annotation))
            }
            PactItem::Fn {
                name,
                parameters,
                return_type,
                default,
            } => {
                let param_str = parameters.iter().join(", ");
                let sig = match return_type {
                    Some(ret) => format!("fn {}({param_str}) -> {}", name, ret),
                    None => format!("fn {}({param_str})", name),
                };
                match default {
                    Some(body) => f.pad(&format!("{sig} {body}")),
                    None => f.pad(&format!("{sig};")),
                }
            }
        }
    }
}
