use crate::{Expr, ExprKind, IntoExpr};

/// Representation of a null coalecence evaluation in mimas.
#[derive(Debug, PartialEq, Clone)]
pub struct Coalescence {
    /// The left hand side of the null coalecence evaluation.
    pub left: Expr,
    /// The right hand side of the null coalecence evaluation.
    pub right: Expr,
}
impl Coalescence {
    /// Creates a new null coalecence evaluation.
    pub(crate) fn new(left: Expr, right: Expr) -> Self {
        Self { left, right }
    }
}
impl From<Coalescence> for ExprKind {
    fn from(null: Coalescence) -> Self {
        Self::Coalescence(null)
    }
}
impl IntoExpr for Coalescence {}

#[mutants::skip]
impl std::fmt::Display for Coalescence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{} ?? {}", self.left, self.right))
    }
}
