use crate::{Expr, ExprKind, IntoExpr, components::Pat};

/// Representation of a for loop in mimas.
#[derive(Debug, PartialEq, Clone)]
pub struct For {
    /// The binding pattern for each element.
    pub binding: Pat,
    /// The iterator that is being looped on.
    pub iterator: Expr,
    /// The body of the loop.
    pub body: Expr,
}
impl For {
    /// Creates a new for loop.
    pub(crate) fn new(
        binding: impl Into<Pat>,
        iterator: impl Into<Expr>,
        body: impl Into<Expr>,
    ) -> Self {
        Self {
            binding: binding.into(),
            iterator: iterator.into(),
            body: body.into(),
        }
    }
}
impl From<For> for ExprKind {
    fn from(for_loop: For) -> Self {
        Self::For(for_loop)
    }
}
impl IntoExpr for For {}

#[mutants::skip]
impl std::fmt::Display for For {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!(
            "for {} in {} {}",
            self.binding, self.iterator, self.body
        ))
    }
}
