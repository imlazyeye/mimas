use crate::{
    Expr,
    components::{Annotation, Binding},
    expr::Ident,
    item::{IntoItem, ItemKind},
};
use itertools::Itertools;
use shared::{Located, Location};

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

#[derive(Debug, Clone)]
pub enum PactItem {
    /// A constant signature: `const NAME: T;` or, with a default value, `const NAME: T = value;`.
    Const {
        name: Ident,
        annotation: Annotation,
        default: Option<Expr>,
        location: Location,
    },
    /// A method signature: `fn name(params) -> ret;` or, with a default body, `fn name(params) ->
    /// ret { body }`. Parameters may not have default values in a pact signature.
    Fn {
        name: Ident,
        parameters: Vec<Binding>,
        return_type: Option<Annotation>,
        /// `Some(body)` if the pact provides a default implementation. Impls may omit this method
        /// when a default exists.
        default: Option<Expr>,
        location: Location,
    },
}

impl Located for PactItem {
    fn location(&self) -> Location {
        match self {
            PactItem::Const { location, .. } | PactItem::Fn { location, .. } => *location,
        }
    }
}

impl PartialEq for PactItem {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                PactItem::Const {
                    name,
                    annotation,
                    default,
                    ..
                },
                PactItem::Const {
                    name: other_name,
                    annotation: other_annotation,
                    default: other_default,
                    ..
                },
            ) => (name, annotation, default) == (other_name, other_annotation, other_default),
            (
                PactItem::Fn {
                    name,
                    parameters,
                    return_type,
                    default,
                    ..
                },
                PactItem::Fn {
                    name: other_name,
                    parameters: other_parameters,
                    return_type: other_return_type,
                    default: other_default,
                    ..
                },
            ) => {
                (name, parameters, return_type, default)
                    == (
                        other_name,
                        other_parameters,
                        other_return_type,
                        other_default,
                    )
            }
            _ => false,
        }
    }
}

#[mutants::skip]
impl std::fmt::Display for PactItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PactItem::Const {
                name,
                annotation,
                default,
                ..
            } => match default {
                Some(value) => f.pad(&format!("const {}: {} = {};", name, annotation, value)),
                None => f.pad(&format!("const {}: {};", name, annotation)),
            },
            PactItem::Fn {
                name,
                parameters,
                return_type,
                default,
                ..
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
