//! lunacade on the page. The page drives a [`Console`] from JavaScript: it loads a cart, calls
//! `frame` 60 times a second with what the player is holding, copies the screen out and reads
//! what the cart printed. Everything that crosses over is a JSON string or a number.
use std::cell::OnceCell;

use lunacade_core::{Cart, Diagnostic, HEIGHT, Input, Library, Machine, WIDTH};
use wasm_bindgen::prelude::*;

mod carts;

thread_local! {
    static LIBRARY: OnceCell<Library<()>> = const { OnceCell::new() };
}

/// A cart, running.
#[wasm_bindgen]
pub struct Console {
    machine: Machine,
}

#[wasm_bindgen]
impl Console {
    /// Loads a cart given as `{files: {path: text}}` and runs its top-level code. `seed` starts
    /// `luna::random`. A cart that doesn't load throws its diagnostics as a JSON array.
    #[wasm_bindgen(constructor)]
    pub fn new(cart_json: &str, seed: f64) -> Result<Console, JsValue> {
        let problems = |list: Vec<Diagnostic>| {
            JsValue::from_str(&serde_json::to_string(&list).unwrap_or_default())
        };
        let cart: Cart = serde_json::from_str(cart_json)
            .map_err(|error| problems(vec![Diagnostic::at("", "", 0..0, error.to_string())]))?;
        let machine = Machine::load(&cart, seed as u64).map_err(problems)?;
        Ok(Console { machine })
    }

    /// Plays one frame. `held` has a bit for each button in the order of `Button`'s variants (up,
    /// down, left, right, A, B, X, Y, Start) and `mouse_held` one for each `Mouse` button (left,
    /// right). The mouse is in screen pixels, and anywhere off the screen counts as no mouse.
    /// Returns whether the frame ran, which is false once the machine has halted.
    pub fn frame(&mut self, held: u16, mouse_x: i32, mouse_y: i32, mouse_held: u8) -> bool {
        let on_screen =
            (0..WIDTH as i32).contains(&mouse_x) && (0..HEIGHT as i32).contains(&mouse_y);
        let input = Input {
            held,
            mouse: on_screen.then_some((mouse_x, mouse_y)),
            mouse_held,
        };
        !self.machine.frame(&input)
    }

    /// Writes the screen into `rgba`, which is `WIDTH * HEIGHT * 4` bytes.
    pub fn screen(&self, rgba: &mut [u8]) {
        self.machine.screen().write_rgba(rgba);
    }

    /// Takes the lines the cart has printed since the last call.
    pub fn take_output(&mut self) -> Vec<String> {
        self.machine.take_output()
    }

    /// The fault that halted the machine as a JSON diagnostic, if one did.
    pub fn fault(&self) -> Option<String> {
        let fault = self.machine.fault()?;
        serde_json::to_string(fault).ok()
    }
}

/// Checks a cart given as `{files: {path: text}}` without running it, and gives its diagnostics as
/// a JSON array.
#[wasm_bindgen]
pub fn check_cart(cart_json: &str) -> Result<String, JsError> {
    let cart: Cart = serde_json::from_str(cart_json)?;
    let list = LIBRARY.with(|library| {
        let library = library.get_or_init(lunacade_core::library);
        lunacade_core::check_with(&cart, library)
    });
    Ok(serde_json::to_string(&list)?)
}

/// Whether a `.mim` file's text makes it a module, which leaves the one that isn't as the cart's
/// script.
#[wasm_bindgen]
pub fn is_module(text: &str) -> bool {
    lunacade_core::is_module(text)
}

/// The carts that ship with lunacade, as a JSON array of `{id, files}`.
#[wasm_bindgen]
pub fn examples() -> String {
    carts::examples()
}
