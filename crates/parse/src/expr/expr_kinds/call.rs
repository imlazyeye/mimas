use itertools::Itertools;

use crate::{Expr, ExprKind, IntoExpr, expr::Ident};

/// Representation of a call expression in mimas.
#[derive(Debug, PartialEq, Clone)]
pub struct Call {
    /// The leftside of the call (the value being invoked).
    pub left: Expr,
    /// The arguments passed into this call.
    pub arguments: Vec<Argument>,
}
impl Call {
    /// Creates a new call.
    pub(crate) fn new(left: Expr, arguments: Vec<Argument>) -> Self {
        Self { left, arguments }
    }
}
impl From<Call> for ExprKind {
    fn from(call: Call) -> Self {
        Self::Call(call)
    }
}
impl IntoExpr for Call {}

#[derive(Debug, PartialEq, Clone)]
pub struct Argument {
    pub name: Option<Ident>,
    pub value: Expr,
}

impl Argument {
    #[cfg(test)]
    pub(crate) fn new(value: Expr) -> Self {
        Self { name: None, value }
    }

    #[cfg(test)]
    pub(crate) fn named(name: Ident, value: Expr) -> Self {
        Self {
            name: Some(name),
            value,
        }
    }
}

#[mutants::skip]
impl std::fmt::Display for Call {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!(
            "{}({})",
            self.left,
            self.arguments.iter().join(", ")
        ))
    }
}

#[mutants::skip]
impl std::fmt::Display for Argument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = self.name.as_ref() {
            f.pad(&format!("{name}={}", self.value))
        } else {
            f.pad(&format!("{}", self.value))
        }
    }
}
