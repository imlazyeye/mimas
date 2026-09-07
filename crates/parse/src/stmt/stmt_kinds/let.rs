use crate::{
    Expr, IntoStmt, StmtKind,
    components::{Annotation, Pat},
};

/// Assignment to a newly bound identifier.
#[derive(Debug, PartialEq, Clone)]
pub struct Let {
    pub left: Pat,
    pub annotation: Option<Annotation>,
    pub right: Expr,
    pub else_branch: Option<Expr>,
}

impl Let {
    /// Creates a new let statement.
    #[cfg(test)]
    pub(crate) fn new(left: Pat, right: Expr, annotation: Option<Annotation>) -> Self {
        Self {
            left,
            right,
            annotation,
            else_branch: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_else(
        left: Pat,
        right: Expr,
        annotation: Option<Annotation>,
        else_branch: Option<Expr>,
    ) -> Self {
        Self {
            left,
            right,
            annotation,
            else_branch,
        }
    }
}
impl From<Let> for StmtKind {
    fn from(stmt: Let) -> Self {
        Self::Let(stmt)
    }
}

impl IntoStmt for Let {}

#[mutants::skip]
impl std::fmt::Display for Let {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!(
            "let {}{} = {}{};",
            self.left,
            self.annotation
                .as_ref()
                .map(|v| format!(": {v}"))
                .unwrap_or_default(),
            self.right,
            self.else_branch
                .as_ref()
                .map_or_else(|| "".into(), |v| format!(" else {v}"))
        ))
    }
}
