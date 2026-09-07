use crate::{Expr, ExprKind, IntoExpr};

/// Representation of a for loop in mimas.
#[derive(Debug, PartialEq, Clone)]
pub struct Range {
    pub start: Expr,
    pub end: Expr,
    pub inclusive: bool,
}
impl Range {
    /// Creates a new for loop.
    pub(crate) fn new(start: impl Into<Expr>, end: impl Into<Expr>, inclusive: bool) -> Self {
        Self {
            start: start.into(),
            end: end.into(),
            inclusive,
        }
    }
}
impl From<Range> for ExprKind {
    fn from(range: Range) -> Self {
        Self::Range(range)
    }
}
impl IntoExpr for Range {}

#[mutants::skip]
impl std::fmt::Display for Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!(
            "{}..{}{}",
            self.start,
            if self.inclusive { "=" } else { "" },
            self.end
        ))
    }
}
