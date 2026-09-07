use crate::{Expr, ExprKind, IntoExpr, op};

/// Representation of a logical expression in mimas.
#[derive(Debug, PartialEq, Clone)]
pub struct Logical {
    /// The left hand side of the logical assessment.
    pub left: Expr,
    /// The operator used in this logical assessment.
    pub op: LogicalOp,
    /// The right hand side of the logical assessment.
    pub right: Expr,
}
impl Logical {
    /// Creates a new logical assessment.
    pub(crate) fn new(left: Expr, op: LogicalOp, right: Expr) -> Self {
        Self { left, op, right }
    }
}
impl From<Logical> for ExprKind {
    fn from(logical: Logical) -> Self {
        Self::Logical(logical)
    }
}
impl IntoExpr for Logical {}

op!(
    /// The various logical operations supported in mimas.
    LogicalOp {
        DoubleAmpersand => And,
        DoublePipe => Or
    }
);

#[mutants::skip]
impl std::fmt::Display for Logical {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{} {} {}", self.left, self.op, self.right))
    }
}
