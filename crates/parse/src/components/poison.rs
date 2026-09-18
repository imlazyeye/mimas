use crate::{ExprKind, ItemKind};

/// The lexeme and rendering every kind of poison shares.
pub const POISON: &str = "<poison>";

/// Stands in for a node that failed to parse. Its error is already among the parser's errors.
/// Later stages treat poison as unreachable, so an Ast holding any is only fit to be thrown
/// away with its errors.
#[derive(Debug, PartialEq, Clone)]
pub struct Poison;

impl Poison {
    /// For the stages after parsing to match poison with. They only ever run on an Ast that
    /// parsed without errors, and that has none.
    pub fn escaped(&self) -> ! {
        unreachable!("poison never leaves the parser")
    }
}

impl From<Poison> for ExprKind {
    fn from(poison: Poison) -> Self {
        Self::Poison(poison)
    }
}

impl From<Poison> for ItemKind {
    fn from(poison: Poison) -> Self {
        Self::Poison(poison)
    }
}

#[mutants::skip]
impl std::fmt::Display for Poison {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(POISON)
    }
}
