use crate::{Expr, ExprKind, IntoExpr};

/// A return statement, with an optional return value.
#[derive(Debug, PartialEq, Clone)]
pub struct Collect {
    /// The value, if any, that this statement returns.
    pub value: Expr,
}
impl Collect {
    /// Creates a new return statement with an optional value.
    pub(crate) fn new<E: Into<Expr>>(value: E) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl From<Collect> for ExprKind {
    fn from(ret: Collect) -> Self {
        Self::Collect(ret)
    }
}
impl IntoExpr for Collect {}

#[mutants::skip]
impl std::fmt::Display for Collect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("collect {}", self.value))
    }
}
