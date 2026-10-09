//! lunacade, a fantasy console for mimas, without a window: its machine, screen and cart format. A
//! front end gives the machine the input each frame, then shows the screen and reads the lines the
//! cart printed.
mod api {
    pub(crate) mod gfx;
    pub(crate) mod input;
    pub(crate) mod install;
    pub(crate) mod luna;
}
mod cart;
mod check;
mod font;
mod input;
mod machine;
mod screen;

pub use api::{
    gfx::Color,
    input::{Button, Mouse},
    install::library,
};
pub use cart::{Cart, CartBuild};
pub use check::{Diagnostic, DiagnosticKind, check, check_with};
pub use host::Library;
pub use input::Input;
pub use machine::Machine;
pub use parse::lex::is_module;
pub use screen::{Clip, PALETTE, Screen};

/// The screen's width in pixels.
pub const WIDTH: usize = 256;

/// The screen's height in pixels.
pub const HEIGHT: usize = 144;

/// The width and height of the sprite sheet in pixels.
pub const SHEET_SIZE: usize = 128;

/// The width and height of a sprite in pixels.
pub const SPRITE_SIZE: usize = 8;

/// The most ops a frame, or a cart's top-level code, can run before it faults with "ran out of
/// fuel".
pub const FUEL: u64 = 5_000_000;
