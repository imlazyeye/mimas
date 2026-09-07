use crate::{Expr, ExprKind, IntoExpr, components::Pat};

/// Representation of a repeat loop in mimas.
#[derive(Debug, PartialEq, Clone)]
pub struct While {
    pub binding: Option<Pat>,
    pub header: Expr,
    pub body: Expr,
}
impl While {
    #[cfg(test)]
    pub(crate) fn new(header: Expr, body: Expr) -> Self {
        Self {
            binding: None,
            header,
            body,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_pat(binding: Pat, header: Expr, body: Expr) -> Self {
        Self {
            binding: Some(binding),
            header,
            body,
        }
    }
}
impl From<While> for ExprKind {
    fn from(while_loop: While) -> Self {
        Self::While(while_loop)
    }
}
impl IntoExpr for While {}

#[mutants::skip]
impl std::fmt::Display for While {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let insert = if let Some(name) = self.binding.as_ref() {
            format!(" let {} =", name)
        } else {
            String::new()
        };
        f.pad(&format!("while{insert} {} {}", self.header, self.body))
    }
}
