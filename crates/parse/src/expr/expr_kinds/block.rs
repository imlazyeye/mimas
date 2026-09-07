use itertools::Itertools;

use crate::{Expr, ExprKind, IntoExpr, Stmt};

/// Representation of a block operation in mimas.
#[derive(Debug, PartialEq, Clone)]
pub struct Block {
    /// The statements contained in this block.
    pub body: Vec<Stmt>,
    /// The expr this block yields (unless the user did not provide one).
    pub yielded_expr: Option<Expr>,
}
impl Block {
    /// Creates a new block expression.
    #[cfg(test)]
    pub(crate) fn new(body: Vec<Stmt>) -> Self {
        Self {
            body,
            yielded_expr: None,
        }
    }

    /// Creates a new block expression that includes a yielded value.
    #[cfg(test)]
    pub(crate) fn new_with_yield(body: Vec<Stmt>, yielded_expr: Expr) -> Self {
        Self {
            body,
            yielded_expr: Some(yielded_expr),
        }
    }
}
impl From<Block> for ExprKind {
    fn from(block: Block) -> Self {
        Self::Block(block)
    }
}
impl IntoExpr for Block {}

#[mutants::skip]
impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let body_str = self.body.iter().map(Stmt::to_string).join(" ");
        if let Some(yielded_expr) = self.yielded_expr.as_ref() {
            f.pad(&format!("{{{body_str}{yielded_expr} }}"))
        } else {
            f.pad(&format!("{{{body_str}}}"))
        }
    }
}
