//! mimas is a statically typed, embeddable scripting language for Rust.
//!
//! This crate is the front door for host programs: it re-exports the pieces of the pipeline you
//! need and bundles the standard library, so embedding is two lines.
//!
//! ```
//! let mut vm = mimas::compile_source(r#"print("hello from mimas");"#).unwrap();
//! vm.run().unwrap();
//! ```
//!
//! # Sharing Rust with a script
//!
//! Tag an item with [`macro@mimas`] and the script can see it, type-checked like anything native.
//! Structs, enums, free functions, and `impl` blocks all work -- note that an `impl` needs the
//! attribute too, not just the type it belongs to.
//!
//! ```
//! use mimas::mimas;
//!
//! #[mimas]
//! struct User(String);
//!
//! #[mimas]
//! impl User {
//!     fn greet(self) -> String {
//!         format!("Hello, {}!", self.0)
//!     }
//! }
//!
//! let mut vm = mimas::compile_source(r#"print(User("mimas").greet());"#).unwrap();
//! vm.run().unwrap();
//! ```
//!
//! Multi-file projects go through [`compile_files`]. For host state, runtime errors, and the rest
//! of the embedding surface, see the [Extension with Rust](https://mim.as/extension-with-rust.html)
//! guide.
//!
//! # Stability
//!
//! mimas is `0.1.0`. The language design is committed, but the Rust-facing API is not stable yet --
//! expect it to move between releases.

pub use library;
pub use vm;

pub use macros::{MimasEnum, MimasStruct, mimas, native};

pub use shared::{Literal, Ty};
pub use vm::{Ctx, Val, Vm, api::Api};

/// Compiles the given source with mimas's std included.
pub fn compile_source(source: &str) -> Result<vm::Vm, vm::ExecuteError> {
    vm::Vm::compile(source, library::std)
}

/// Compiles the given files with mimas's std included. Files should be passed in within a tuple
/// first containing their name then followed by their source.
pub fn compile_files(files: &[(&str, &str)]) -> Result<vm::Vm, vm::ExecuteError> {
    vm::Vm::compile_files(files, library::std)
}
