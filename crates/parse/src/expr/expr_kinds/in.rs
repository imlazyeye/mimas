use crate::{Expr, ExprKind, IntoExpr};

/// Representation of a membership test in mimas: `x in y`.
#[derive(Debug, PartialEq, Clone)]
pub struct In {
    pub left: Expr,
    pub right: Expr,
    pub condition: bool,
}

impl In {
    pub(crate) fn new(left: Expr, right: Expr, condition: bool) -> Self {
        Self {
            left,
            right,
            condition,
        }
    }
}

impl From<In> for ExprKind {
    fn from(v: In) -> Self {
        Self::In(v)
    }
}

impl IntoExpr for In {}

#[mutants::skip]
impl std::fmt::Display for In {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!(
            "{} {}in {}",
            self.left,
            if self.condition { "" } else { "!" },
            self.right
        ))
    }
}
