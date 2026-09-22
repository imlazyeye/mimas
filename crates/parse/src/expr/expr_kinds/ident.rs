use crate::{NodeId, expr::*};
use shared::{Located, Location, Span};

/// Representation of an identifier in mimas, which could be any variable.
#[derive(Debug, Clone, Eq)]
pub struct Ident {
    /// The name of this identifier
    pub lexeme: String,
    /// The location of the original token.
    pub location: Location,
    /// Lets the solver record what the name resolved to (tooling reads it, the compile doesn't).
    pub id: NodeId,
}
impl Ident {
    /// Creates a new identifier.
    pub fn new(lexeme: impl Into<String>, location: Location) -> Self {
        Self {
            lexeme: lexeme.into(),
            location,
            id: NodeId::new(),
        }
    }

    /// Used when identifiers are needed from a source outside of the user code.
    pub fn synthetic(lexeme: impl Into<String>) -> Self {
        Self::new(lexeme, Location::new(0, Span::SYNTHETIC))
    }

    /// Whether or not this ident is referencing the identity type.
    pub fn is_identity(&self) -> bool {
        matches!(self.lexeme.as_str(), "self" | "Self")
    }
}

impl From<Ident> for String {
    fn from(value: Ident) -> Self {
        value.lexeme
    }
}

impl From<Ident> for ExprKind {
    fn from(iden: Ident) -> Self {
        Self::Ident(iden)
    }
}

impl IntoExpr for Ident {}

impl Located for Ident {
    fn location(&self) -> Location {
        self.location
    }
}

#[mutants::skip]
impl std::fmt::Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&self.lexeme)
    }
}

impl PartialEq<Ident> for Ident {
    fn eq(&self, other: &Ident) -> bool {
        self.lexeme == other.lexeme
    }
}

impl std::hash::Hash for Ident {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.lexeme.hash(state);
    }
}
