use itertools::Itertools;

use crate::{
    Expr, ExprKind, IntoExpr,
    components::{Annotation, Binding},
};

/// Representation of Closure declaration in mimas.
#[derive(Debug, PartialEq, Clone)]
pub struct Closure {
    /// The parameters of this Closure.
    pub parameters: Vec<Binding>,
    /// The body of the Closure declaration.
    pub body: Expr,
    /// The return annotation, if any.
    pub return_type: Option<Annotation>,
}
impl Closure {
    /// Creates a new Closure declaration.
    #[cfg(test)]
    pub(crate) fn new(parameters: Vec<Binding>, body: Expr) -> Self {
        Self {
            parameters,
            body,
            return_type: None,
        }
    }
}
impl From<Closure> for ExprKind {
    fn from(closure: Closure) -> Self {
        Self::Closure(closure)
    }
}
impl IntoExpr for Closure {}

#[mutants::skip]
impl std::fmt::Display for Closure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let param_str = self.parameters.iter().join(", ");
        f.pad(&format!("|{param_str}| {}", self.body))
    }
}
