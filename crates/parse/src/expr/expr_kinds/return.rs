use crate::{Expr, ExprKind, IntoExpr};

/// A return statement, with an optional return value.
#[derive(Debug, PartialEq, Clone)]
pub struct Return {
    /// The value, if any, that this statement returns.
    pub value: Option<Expr>,
}
impl Return {
    /// Creates a new return statement with an optional value.
    pub(crate) fn new(value: Option<Expr>) -> Self {
        Self { value }
    }
}
impl From<Return> for ExprKind {
    fn from(ret: Return) -> Self {
        Self::Return(ret)
    }
}
impl IntoExpr for Return {}

#[mutants::skip]
impl std::fmt::Display for Return {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!(
            "return{}",
            self.value
                .as_ref()
                .map(|v| format!(" {}", v))
                .unwrap_or_default()
        ))
    }
}
