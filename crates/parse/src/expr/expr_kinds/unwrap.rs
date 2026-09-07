use crate::{Expr, ExprKind, IntoExpr};

/// An unwrap expression (`expr?!`) in mimas.
#[derive(Debug, PartialEq, Clone)]
pub struct Unwrap {
    /// The expression being unwrapped.
    pub expr: Expr,
}

impl From<Unwrap> for ExprKind {
    fn from(unwrap: Unwrap) -> Self {
        Self::Unwrap(unwrap)
    }
}
impl IntoExpr for Unwrap {}

#[mutants::skip]
impl std::fmt::Display for Unwrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{}?!", self.expr))
    }
}
