mod library;
#[cfg(feature = "serde")]
mod manifest;
mod project;
mod records;
mod registry;

pub use library::*;
#[cfg(feature = "serde")]
pub use manifest::*;
pub use project::*;
pub use records::*;
pub use registry::*;

shared::id!(pub NativeId);

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Intrinsic {
    Len,
    In,
    Push,
    ToFloat,
    Sqrt,
    File,
}
