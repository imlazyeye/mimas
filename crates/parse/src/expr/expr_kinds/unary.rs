use crate::{Expr, ExprKind, IntoExpr, op};

/// Representation of a unary operation in mimas.
#[derive(Debug, PartialEq, Clone)]
pub struct Unary {
    /// The unary operator.
    pub op: UnaryOp,
    /// The right hand side of the unary operation.
    pub right: Expr,
}
impl Unary {
    /// Creates a new unary operation.
    pub(crate) fn new(op: UnaryOp, right: Expr) -> Self {
        Self { op, right }
    }
}
impl From<Unary> for ExprKind {
    fn from(unary: Unary) -> Self {
        Self::Unary(unary)
    }
}
impl IntoExpr for Unary {}

op!(
    /// The various unary operations supported in mimas.
    UnaryOp {
        Minus => Negative,
        Plus => Positive,
        Tilde => BitwiseNot,
        Bang => Not,
    }
);

#[mutants::skip]
impl std::fmt::Display for Unary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{}{}", self.op, self.right))
    }
}
