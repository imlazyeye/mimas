use crate::{Expr, components::*};
use shared::{Located, Location, Span};

#[derive(Debug, PartialEq, Clone)]
pub struct Binding {
    pub left: Pat,
    pub right: Option<Expr>,
    pub annotation: Option<Annotation>,
}
impl Binding {
    pub(crate) fn new(left: impl Into<Pat>) -> Self {
        Self {
            left: left.into(),
            right: None,
            annotation: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_annotation(mut self, annotation: Annotation) -> Self {
        self.annotation = Some(annotation);
        self
    }

    #[cfg(test)]
    pub(crate) fn with_initilization(mut self, right: Expr) -> Self {
        self.right = Some(right);
        self
    }

    pub fn location(&self) -> Location {
        Location::new(
            self.left.file_id(),
            Span::new(
                self.left.span().start(),
                self.right
                    .as_ref()
                    .map_or_else(|| self.left.span().end(), |v| v.span().end()),
            ),
        )
    }
}

#[mutants::skip]
impl std::fmt::Display for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let left = &self.left;
        match (&self.right, &self.annotation) {
            (None, None) => f.pad(&format!("{left}")),
            (Some(right), None) => f.pad(&format!("{left} = {right}")),
            (None, Some(annotation)) => f.pad(&format!("{left}: {annotation}")),
            (Some(right), Some(annotation)) => f.pad(&format!("{left}: {annotation} = {right}")),
        }
    }
}
