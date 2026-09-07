#![warn(clippy::dbg_macro)]
#![warn(clippy::map_unwrap_or)]
#![warn(clippy::similar_names)]
#![warn(clippy::todo)]
#![warn(clippy::undocumented_unsafe_blocks)]
// temporary
#![allow(clippy::result_large_err)]

pub mod components {
    mod adt;
    mod control_flow;
    mod rib;
    mod ty;
    pub use adt::*;
    pub use control_flow::*;
    pub use rib::*;
    pub use ty::*;
}

pub mod traits {
    mod hoist;
    mod query;
    mod solve;
    pub(crate) use hoist::*;
    pub use query::*;
    pub use solve::*;
}

pub mod utils {
    mod macros;
    mod printer;
    mod reduce;
    pub use printer::*;
    pub use reduce::*;
}

mod exhaustion;
mod resolutions;
mod solver;
mod unify;
pub use resolutions::*;
pub use solver::*;
use unify::*;

pub mod errors;
pub use errors::{Error, Result};

#[cfg(test)]
mod tests {
    #[macro_use]
    pub mod solve_test_utils;

    mod reduce_exprs;
    mod solve_enums;
    mod solve_exprs;
    mod solve_functions;
    mod solve_match;
    mod solve_modules;
    mod solve_options;
    mod solve_pacts;
    mod solve_results;
    mod solve_structs;
    mod solve_variables;
}
