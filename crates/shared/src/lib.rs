mod id;
mod interner;
mod literal;
mod location;
mod ty;

pub use id::*;
pub use interner::*;
pub use literal::*;
pub use location::*;
pub use ty::*;

pub type Error = miette::Report;
pub type Result<T> = std::result::Result<T, Error>;

id!(pub AdtId);
id!(pub BodyId);
