use std::{collections::HashMap, sync::Arc};

use compile::BinOp;
use miette::{Diagnostic, NamedSource, SourceSpan};
use shared::FileId;
use thiserror::Error;

use crate::{Captured, Val};

pub use shared::{Error, Result};

pub type RtResult<T> = std::result::Result<T, RtErr>;

/// Per-file source map. Built once at vm setup; the dispatch loop looks up the active file's
/// `NamedSource` here when constructing a [`RuntimeError`].
pub type Sources = HashMap<FileId, NamedSource<Arc<str>>>;

/// Span-attached runtime diagnostic. The active bytecode chunk supplies the span at the
/// dispatch boundary; the kind carries the data needed to format the label text.
#[derive(Error, Debug, Diagnostic)]
#[error("{kind}")]
pub struct LocatedRtErr {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("{}", self.kind.tag())]
    pub at: SourceSpan,
    pub kind: RtErr,
}

/// All possible runtime faults. Span-less by design -- `step()` doesn't have a span at the
/// throw site, only the data describing what went wrong.
#[derive(Error, Debug)]
pub enum RtErr {
    #[error("match did not match on any provided patterns")]
    MatchPanicReached,

    #[error("divided by zero")]
    DivByZero,

    #[error("took the remainder by zero")]
    ModByZero,

    #[error("shift amount out of range")]
    InvalidShift,

    #[error("index out of bounds")]
    IndexOutOfBounds,

    #[error("unwrapped a null value")]
    UnwrappedNull,

    #[error("unwrapped a raised error: {0}")]
    UnwrappedRaised(String),

    #[error("operator received incompatible operand types")]
    InvalidBinOperands {
        lhs: Captured,
        op: BinOp,
        rhs: Captured,
    },

    #[error("unary operator received an incompatible operand type")]
    InvalidUnaryOperand,

    #[error("value cannot be indexed with this key")]
    InvalidIndexTarget { set: Captured, index: Captured },

    #[error("integer arithmetic overflowed")]
    IntegerOverflow,

    #[error("user-triggered panic")]
    UserPanic,

    #[error("{0}")]
    InvalidArgument(String),

    #[error("{0}")]
    Custom(String),
}

impl RtErr {
    /// Text for the primary label, paired with the bytecode-derived anchor by the dispatch
    /// loop. Most kinds get a generic "here"; richer kinds inline operand info.
    pub(crate) fn tag(&self) -> String {
        match self {
            Self::InvalidBinOperands { lhs, op, rhs } => {
                format!("cannot apply `{op}` to `{lhs}` and `{rhs}`")
            }
            Self::InvalidIndexTarget { set, index } => {
                format!("`{set}` cannot be indexed with `{index}`")
            }
            Self::MatchPanicReached => "no arm matched this value".to_string(),
            Self::DivByZero => "the divisor is zero".to_string(),
            Self::ModByZero => "the right-hand side is zero".to_string(),
            Self::InvalidShift => "shifts take an unsigned 32 bit integer".to_string(),
            Self::IndexOutOfBounds => "this index is past the end".to_string(),
            Self::UnwrappedNull => "this unwrap found null".to_string(),
            Self::UnwrappedRaised(_) => "this unwrap found a raised error".to_string(),
            Self::InvalidUnaryOperand => "this operand does not support the operator".to_string(),
            Self::IntegerOverflow => "this arithmetic overflows a 64 bit integer".to_string(),
            Self::UserPanic => "panicked here".to_string(),
            Self::InvalidArgument(_) | Self::Custom(_) => "here".to_string(),
        }
    }
}

impl RtErr {
    pub fn invalid_bin<'gc>(lhs: Val<'gc>, op: BinOp, rhs: Val<'gc>) -> Self {
        Self::InvalidBinOperands {
            lhs: lhs.capture(),
            op,
            rhs: rhs.capture(),
        }
    }

    pub fn invalid_index<'gc>(set: Val<'gc>, index: Val<'gc>) -> Self {
        Self::InvalidIndexTarget {
            set: set.capture(),
            index: index.capture(),
        }
    }
}
