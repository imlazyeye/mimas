use crate::{
    Expr, Ident,
    components::Annotation,
    item::{IntoItem, ItemKind},
};

/// `const` declaration -- a name bound to a compile-time-foldable expression. Visibility lives
/// on the wrapping [Item].
#[derive(Debug, PartialEq, Clone)]
pub struct Const {
    pub left: Ident,
    pub annotation: Option<Annotation>,
    pub right: Expr,
}

impl Const {
    /// Creates a new const declaration.
    #[cfg(test)]
    pub(crate) fn new(left: Ident, right: Expr, annotation: Option<Annotation>) -> Self {
        Self {
            left,
            right,
            annotation,
        }
    }
}

impl From<Const> for ItemKind {
    fn from(con: Const) -> Self {
        Self::Const(con)
    }
}
impl IntoItem for Const {}

#[mutants::skip]
impl std::fmt::Display for Const {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!(
            "const {}{} = {};",
            self.left,
            self.annotation
                .as_ref()
                .map(|v| format!(": {v}"))
                .unwrap_or_default(),
            self.right,
        ))
    }
}
