use crate::{ExprKind, IntoExpr};

/// A `continue` statement.
#[derive(Debug, PartialEq, Clone)]
pub struct Continue;
impl From<Continue> for ExprKind {
    fn from(con: Continue) -> Self {
        Self::Continue(con)
    }
}
impl IntoExpr for Continue {}

#[mutants::skip]
impl std::fmt::Display for Continue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad("continue")
    }
}
