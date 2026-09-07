use itertools::Itertools;

use crate::{
    Ident,
    item::{IntoItem, Item, ItemKind},
};

/// Representation of an impl block in mimas.
#[derive(Debug, PartialEq, Clone)]
pub struct Impl {
    /// The type this impl block is for.
    pub target: Ident,
    /// The pact being implemented, if any.
    pub pact: Option<Ident>,
    /// The items this impl declares.
    pub items: Vec<Item>,
}
impl Impl {
    pub(crate) fn new(target: Ident, pact: Option<Ident>, items: Vec<Item>) -> Self {
        Self {
            target,
            pact,
            items,
        }
    }
}
impl From<Impl> for ItemKind {
    fn from(imp: Impl) -> Self {
        Self::Impl(imp)
    }
}
impl IntoItem for Impl {}

#[mutants::skip]
impl std::fmt::Display for Impl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let head = match &self.pact {
            Some(pact) => format!("impl {} for {}", pact, self.target),
            None => format!("impl {}", self.target),
        };
        if self.items.is_empty() {
            f.pad(&format!("{head} {{}}"))
        } else {
            f.pad(&format!("{head} {{ {} }}", self.items.iter().join(" ")))
        }
    }
}
