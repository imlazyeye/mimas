use crate::{IntoStmt, NodeId, StmtKind};

use super::*;
use shared::{Located, Location};

/// Declares all the various needed pieces of the ItemKinds.
macro_rules! declare_item_kinds {
    (#[doc = $doc:expr]$name:ident { $($tok:ident), * $(,)? }) => {
        #[derive(Debug, PartialEq, Clone)]
        pub enum $name {
            $(
                $tok($tok),
            )*
        }

        impl IntoItem for $name {}

        #[mutants::skip]
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                 match self {
                    $($name::$tok(v) => std::fmt::Display::fmt(v, f),)*
                }
            }
        }
    };
}

declare_item_kinds!(
    /// Items -- declarations that name something into a scope. Hoisted before bodies are solved.
    ItemKind {
        Function,
        Struct,
        Pact,
        Enum,
        Impl,
        Const,
        Use,
    }
);

/// A wrapper around an [ItemKind], containing additional information discovered while parsing.
/// Visibility (`pub`) lives here uniformly, not duplicated across the inner kinds.
#[derive(Debug, Clone)]
pub struct Item {
    kind: Box<ItemKind>,
    id: NodeId,
    location: Location,
    public: bool,
}

impl Item {
    /// Creates a new item.
    pub fn new(kind: ItemKind, location: Location, public: bool) -> Self {
        Self {
            kind: Box::new(kind),
            id: NodeId::new(),
            location,
            public,
        }
    }

    /// Get a reference to the inner ItemKind.
    pub fn kind(&self) -> &ItemKind {
        self.kind.as_ref()
    }

    /// Get the item's id.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Whether this item is `pub`.
    pub fn public(&self) -> bool {
        self.public
    }
}

impl Located for Item {
    fn location(&self) -> Location {
        self.location
    }
}

impl From<Item> for StmtKind {
    fn from(item: Item) -> Self {
        Self::Item(item)
    }
}
impl IntoStmt for Item {}

#[mutants::skip]
impl std::fmt::Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.public {
            f.write_str("pub ")?;
        }
        std::fmt::Display::fmt(self.kind(), f)
    }
}

/// Mirror of [crate::IntoExpr] / [crate::IntoStmt] -- anything convertible into an [ItemKind] can
/// be wrapped into an [Item] with a default location for tests.
pub trait IntoItem: Sized + Into<ItemKind> {
    fn into_item(self) -> Item
    where
        Self: Sized,
    {
        Item::new(self.into(), Default::default(), false)
    }
}

impl PartialEq<Item> for Item {
    fn eq(&self, other: &Item) -> bool {
        self.kind == other.kind && self.public == other.public
    }
}
