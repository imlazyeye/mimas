use crate::{Expr, ExprKind, IntoExpr};

/// A return statement, with an optional return value.
#[derive(Debug, PartialEq, Clone)]
pub struct Break {
    /// The value, if any, that this statement returns.
    pub value: Option<Expr>,
}
impl Break {
    /// Creates a new return statement with an optional value.
    pub(crate) fn new(value: Option<Expr>) -> Self {
        Self { value }
    }
}
impl From<Break> for ExprKind {
    fn from(ret: Break) -> Self {
        Self::Break(ret)
    }
}
impl IntoExpr for Break {}

#[mutants::skip]
impl std::fmt::Display for Break {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!(
            "break{}",
            self.value
                .as_ref()
                .map(|v| format!(" {}", v))
                .unwrap_or_default()
        ))
    }
}
