use crate::{Expr, ExprKind, IntoExpr};

#[derive(Debug, PartialEq, Clone)]
pub struct Raise {
    pub value: Expr,
}

impl Raise {
    pub(crate) fn new(value: Expr) -> Self {
        Self { value }
    }
}

impl From<Raise> for ExprKind {
    fn from(raise: Raise) -> Self {
        Self::Raise(raise)
    }
}
impl IntoExpr for Raise {}

#[mutants::skip]
impl std::fmt::Display for Raise {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("raise {}", self.value))
    }
}
