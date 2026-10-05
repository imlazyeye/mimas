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
pub type Sources = std::collections::HashMap<FileId, miette::NamedSource<std::sync::Arc<str>>>;

id!(pub AdtId);
id!(pub BodyId);
