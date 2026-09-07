use crate::{Expr, ExprKind, IntoExpr, components::Pat};

/// Representation of an if statement in mimas.
///
/// I'm aware that its absolutely chaotic that I named this thing `If`.
#[derive(Debug, PartialEq, Clone)]
pub struct If {
    /// The condition this if statement is checking for.
    pub condition: Expr,
    /// The body of the if statement.
    pub main_body: Expr,
    /// The statement attached to this if statement as an else path.
    pub else_expr: Option<Expr>,
    /// The binding for `if let` exprs.
    pub binding: Option<Pat>,
}
impl If {
    /// Creates a new if statement.
    #[cfg(test)]
    pub(crate) fn new(condition: Expr, body: Expr) -> Self {
        Self {
            condition,
            main_body: body,
            else_expr: None,
            binding: None,
        }
    }

    /// Creates a new if statement with an else statement.
    #[cfg(test)]
    pub(crate) fn new_with_else(condition: Expr, body: Expr, else_expr: Expr) -> Self {
        Self {
            condition,
            main_body: body,
            else_expr: Some(else_expr),
            binding: None,
        }
    }
}
impl From<If> for ExprKind {
    fn from(if_stmt: If) -> Self {
        Self::If(if_stmt)
    }
}
impl IntoExpr for If {}

#[mutants::skip]
impl std::fmt::Display for If {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!(
            "if {} {} {}",
            self.condition,
            self.main_body,
            self.else_expr
                .as_ref()
                .map(|e| format!(" else {e}"))
                .unwrap_or_default()
        ))
    }
}
