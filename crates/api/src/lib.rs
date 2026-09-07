mod library;
mod records;
mod registry;

pub use library::*;
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
}
