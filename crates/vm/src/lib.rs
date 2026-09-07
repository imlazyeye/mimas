mod utils;
mod vm;

pub use utils::*;
pub use vm::*;

mod errors;
mod val;
pub use errors::*;
pub use val::*;
pub mod adt;
pub mod anon;
pub mod api;
pub mod conversion;
pub mod freeze;
mod heap;
pub use heap::*;
mod native;
pub use native::*;
pub mod fixtures;
pub use fixtures::*;

// re-exports the MimasEnum / MimasStruct derives resolve against -- saves user crates from
// depending on mimas-api / mimas-shared directly.
pub use ::api::{AdtBinding, ApiAdtKind, ApiVariantFields, Registry};
pub use ::macros::{MimasEnum, MimasStruct, mimas, native};
pub use ::shared::{Literal, Ty};

// the `#[mimas]` attribute macro expands to `vm::inventory::submit!{ ... }`, so the inventory
// crate has to be reachable through `vm`. re-exporting it here means downstream crates that
// author natives only need a `vm` dependency, never a direct `inventory` one.
pub use ::inventory;
