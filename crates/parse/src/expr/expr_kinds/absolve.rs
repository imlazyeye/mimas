use crate::{Expr, ExprKind, IntoExpr};

#[derive(Debug, PartialEq, Clone)]
pub struct Absolve {
    pub left: Expr,
    pub handler: Expr,
}

impl Absolve {
    pub(crate) fn new(left: Expr, handler: Expr) -> Self {
        Self { left, handler }
    }
}

impl From<Absolve> for ExprKind {
    fn from(absolve: Absolve) -> Self {
        Self::Absolve(absolve)
    }
}
impl IntoExpr for Absolve {}

#[mutants::skip]
impl std::fmt::Display for Absolve {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{} absolve {}", self.left, self.handler))
    }
}
