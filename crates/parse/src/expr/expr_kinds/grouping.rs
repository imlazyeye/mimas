use crate::{Expr, ExprKind, IntoExpr};

/// Representation of a grouping in mimas.
#[derive(Debug, PartialEq, Clone)]
pub struct Grouping {
    /// The inner expression contained by this grouping.
    pub inner: Expr,
}
impl Grouping {
    /// Creates a new grouping.
    pub(crate) fn new(inner: Expr) -> Self {
        Self { inner }
    }
    /// Creates a new grouping with lazily generated toks.
    #[cfg(test)]
    pub(crate) fn lazy(inner: Expr) -> Self {
        Self { inner }
    }
}
impl From<Grouping> for ExprKind {
    fn from(grouping: Grouping) -> Self {
        Self::Grouping(grouping)
    }
}
impl IntoExpr for Grouping {}

#[mutants::skip]
impl std::fmt::Display for Grouping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("({})", self.inner))
    }
}
