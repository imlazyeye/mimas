use crate::lex::TokKind;

use crate::{Expr, ExprKind, IntoExpr, expr::Ident};

/// Representation of an access in mimas, such as an array lookup, or dot-notation.
#[derive(Debug, PartialEq, Clone)]
pub enum Access {
    /// Accessing the current scope via `self`. (This would be called `Self`, but it's reserved by
    /// rust.)
    Identity {
        /// The value being extracted from the local scope.
        right: Ident,
    },
    /// Dot access with any struct or object.
    Dot {
        /// The value being accessed.
        left: Expr,
        /// The value being extracted from the leftside value.
        right: Expr,
        kind: AccessKind,
    },
    DoubleColon {
        /// The value being accessed.
        left: Expr,
        /// The value being extracted from the leftside value.
        right: Ident,
    },
    /// Array access. The bool at the end represents if the `@` accessor is present, which denotes
    /// the access to be direct instead of copy-on-write.
    ///
    /// This syntax does not ultimately decide what runtime behavior actually gets applied:
    /// GameMaker has added the option to make *all* array accesses copy-on-write, potentially
    /// marking this syntax for deprecation in the future.
    ///
    /// Both variants have an optional second value, since 2d arrays are still supported (though
    /// deprecated).
    Square {
        /// The array being accessed.
        left: Expr,
        /// The first index supplied to the access.
        key: Expr,
        kind: AccessKind,
    },
}
impl From<Access> for ExprKind {
    fn from(access: Access) -> Self {
        Self::Access(access)
    }
}
impl IntoExpr for Access {}

#[mutants::skip]
impl std::fmt::Display for Access {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Access::Identity { right } => f.pad(&format!("self.{right}")),
            Access::Dot { left, right, kind } => match kind {
                AccessKind::Direct => f.pad(&format!("{left}.{right}")),
                AccessKind::Option => f.pad(&format!("{left}?.{right}")),
            },
            Access::DoubleColon { left, right } => f.pad(&format!("{left}::{right}")),
            Access::Square { left, key, kind } => match kind {
                AccessKind::Direct => f.pad(&format!("{left}[{key}]")),
                AccessKind::Option => f.pad(&format!("{left}?[{key}]")),
            },
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Copy, Clone, Eq)]
pub enum AccessKind {
    Direct,
    Option,
}

impl TryFrom<TokKind<'_>> for AccessKind {
    type Error = ();

    fn try_from(value: TokKind<'_>) -> Result<Self, Self::Error> {
        match value {
            TokKind::Dot | TokKind::LeftSquare => Ok(AccessKind::Direct),
            TokKind::HookDot | TokKind::HookLeftSquare => Ok(AccessKind::Option),
            _ => Err(()),
        }
    }
}
