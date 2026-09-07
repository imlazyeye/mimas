use crate::{Expr, ExprKind, IntoExpr};

/// Representation of a repeat loop in mimas.
#[derive(Debug, PartialEq, Clone)]
pub struct Loop {
    /// The body of the loop.
    pub body: Expr,
}
impl Loop {
    /// Creates a new repeat loop.
    pub(crate) fn new(body: Expr) -> Self {
        Self { body }
    }
}
impl From<Loop> for ExprKind {
    fn from(repeat_loop: Loop) -> Self {
        Self::Loop(repeat_loop)
    }
}
impl IntoExpr for Loop {}

#[mutants::skip]
impl std::fmt::Display for Loop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("loop {}", self.body))
    }
}
