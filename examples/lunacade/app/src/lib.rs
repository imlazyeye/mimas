//! lunacade on the page. The page drives a [`Console`] from JavaScript: it loads a cart, calls
//! `frame` 60 times a second with what the player is holding, copies the screen out and reads
//! what the cart printed. The editor's checks, hovers and renames come from the language server's
//! [`Analysis`] of the cart. Everything that crosses over is a JSON string or a number.
use std::{
    cell::{OnceCell, RefCell},
    collections::BTreeMap,
    path::PathBuf,
    rc::Rc,
};

use lsp::{
    Analysis,
    lsp_types::{Position, Range, TextEdit, Uri},
};
use lunacade_core::{Cart, Diagnostic, DiagnosticKind, HEIGHT, Input, Library, Machine, WIDTH};
use serde::Serialize;
use wasm_bindgen::prelude::*;

mod carts;
mod pixels;

thread_local! {
    static LIBRARY: OnceCell<Rc<Library<()>>> = const { OnceCell::new() };
    /// The analysis of the cart `check_cart` last saw, which the editor's questions are about.
    static ANALYSIS: RefCell<Option<Analysis>> = const { RefCell::new(None) };
}

/// A cart, running.
#[wasm_bindgen]
pub struct Console {
    machine: Machine,
    /// Only the editor exports its sheet. A running game's sprite mutations stay local.
    saved_sheet: Option<pixels::Sheet>,
    sheet_clean: bool,
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
        Ok(Console {
            machine,
            saved_sheet: None,
            sheet_clean: true,
        })
    }

    /// Opens the built-in pixel editor cart on a working copy of `sprites.txt`. Invalid text
    /// is ignored in the preview, and preserved in the file until the user actually edits it.
    pub fn pixel_editor(text: &str) -> Result<Console, JsValue> {
        let (cart, sheet, clean) = pixels::cart(text);
        let mut console = Self::new(&serde_json::to_string(&cart).unwrap(), 0.0)?;
        console.saved_sheet = Some(sheet);
        console.sheet_clean = clean;
        Ok(console)
    }

    /// Whether the editor's sheet can represent all the text it was opened with.
    pub fn sheet_is_clean(&self) -> bool {
        self.sheet_clean
    }

    /// Takes a changed editor sheet as `sprites.txt`, or nothing if no pixels changed. The
    /// page writes this through its usual file store, so autosave, sharing and auto-run agree.
    pub fn take_sheet(&mut self) -> Option<String> {
        let saved = self.saved_sheet.as_mut()?;
        let screen = self.machine.screen();
        if *saved == screen.sheet {
            return None;
        }
        *saved = screen.sheet;
        self.sheet_clean = true;
        Some(pixels::serialize(saved))
    }

    /// Plays one frame. `held` has a bit for each button in the order of `Button`'s variants (up,
    /// down, left, right, A, B, X, Y, Start) and `mouse_held` one for each `Mouse` button (left,
    /// right). `pressed`, `released` and `mouse_pressed` keep events between frames, including
    /// taps that are no longer held. The mouse is in screen pixels, and anywhere off the screen
    /// counts as no mouse.
    /// Returns whether the frame ran, which is false once the machine has halted.
    #[expect(
        clippy::too_many_arguments,
        reason = "Pass scalar input directly across the wasm boundary"
    )]
    pub fn frame(
        &mut self,
        held: u16,
        pressed: u16,
        released: u16,
        mouse_x: i32,
        mouse_y: i32,
        mouse_held: u8,
        mouse_pressed: u8,
    ) -> bool {
        let on_screen =
            (0..WIDTH as i32).contains(&mouse_x) && (0..HEIGHT as i32).contains(&mouse_y);
        let input = Input {
            held,
            pressed,
            released,
            mouse: on_screen.then_some((mouse_x, mouse_y)),
            mouse_held,
            mouse_pressed,
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
/// a JSON array. The analysis it builds answers [`hover`], [`definition`] and the rest until the
/// next check.
#[wasm_bindgen]
pub fn check_cart(cart_json: &str) -> Result<String, JsError> {
    let cart: Cart = serde_json::from_str(cart_json)?;
    let library = LIBRARY.with(|library| {
        library
            .get_or_init(|| Rc::new(lunacade_core::library()))
            .clone()
    });
    // An invalid cart can have several scripts, which the core checks independently. There is
    // no single program for the editor's queries until the cart's structure is valid again.
    if cart.validate().is_err() {
        ANALYSIS.with(|slot| *slot.borrow_mut() = None);
        return Ok(serde_json::to_string(&lunacade_core::check_with(
            &cart, &library,
        ))?);
    }
    let mut problems = Vec::new();
    // the analysis turns paths into uris, which takes paths that look absolute, so every path
    // in it carries a leading `/`
    let files = cart
        .sources()
        .into_iter()
        .map(|(path, text)| (PathBuf::from(format!("/{path}")), text.to_owned()))
        .collect();
    let analysis = Analysis::load(files, library);
    let prefixed = Cart {
        files: cart
            .files
            .iter()
            .map(|(path, text)| (format!("/{path}"), text.clone()))
            .collect(),
    };
    for error in analysis.errors() {
        let mut problem = Diagnostic::from_report(error, &prefixed, DiagnosticKind::Error);
        problem.file = problem.file.trim_start_matches('/').to_owned();
        problems.push(problem);
    }
    ANALYSIS.with(|slot| *slot.borrow_mut() = Some(analysis));
    Ok(serde_json::to_string(&problems)?)
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

// the editor's questions, each about the cart `check_cart` last saw. Positions are the
// protocol's: 0-based lines and UTF-16 columns.

/// What the name at a position means, as an LSP `Hover` in JSON: markdown in `contents.value`
/// and the `range` it covers.
#[wasm_bindgen]
pub fn hover(file: &str, line: u32, character: u32) -> Option<String> {
    ask(|analysis| analysis.hover(&at(file), Position { line, character }))
}

/// Where the name at a position is declared, as `{file, range}` in JSON.
#[wasm_bindgen]
pub fn definition(file: &str, line: u32, character: u32) -> Option<String> {
    ask(|analysis| {
        let location = analysis.definition(&at(file), Position { line, character })?;
        Some(Place {
            file: file_of(&location.uri)?,
            range: location.range,
        })
    })
}

/// Every use of the name at a position in its own file, as a JSON array of ranges.
#[wasm_bindgen]
pub fn highlights(file: &str, line: u32, character: u32) -> Option<String> {
    ask(|analysis| {
        let list = analysis.highlights(&at(file), Position { line, character })?;
        Some(list.into_iter().map(|h| h.range).collect::<Vec<_>>())
    })
}

/// The type of every `let` written without one, as a JSON array of `{position, label}`.
#[wasm_bindgen]
pub fn inlay_hints(file: &str) -> Option<String> {
    let whole = Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: u32::MAX,
            character: 0,
        },
    };
    ask(|analysis| analysis.inlay_hints(&at(file), whole))
}

/// The range of the name at a position, as JSON, when it's one that can be renamed.
#[wasm_bindgen]
pub fn prepare_rename(file: &str, line: u32, character: u32) -> Option<String> {
    ask(|analysis| analysis.prepare_rename(&at(file), Position { line, character }))
}

/// Renames the name at a position everywhere, as `{changes: {file: [edits]}}` in JSON, or
/// `{error}` with why it can't be.
#[wasm_bindgen]
pub fn rename(file: &str, line: u32, character: u32, new_name: &str) -> String {
    let renamed = ANALYSIS.with(|slot| match slot.borrow().as_ref() {
        None => Renamed::Error {
            error: "the cart hasn't been checked yet".to_owned(),
        },
        Some(analysis) => {
            match analysis.rename(&at(file), Position { line, character }, new_name) {
                Ok(edit) => Renamed::Changes {
                    changes: edit
                        .changes
                        .into_iter()
                        .flatten()
                        .filter_map(|(uri, edits)| Some((file_of(&uri)?, edits)))
                        .collect(),
                },
                Err(error) => Renamed::Error { error },
            }
        }
    });
    serde_json::to_string(&renamed).unwrap_or_default()
}

#[derive(Serialize)]
struct Place {
    file: String,
    range: Range,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Renamed {
    Changes {
        changes: BTreeMap<String, Vec<TextEdit>>,
    },
    Error {
        error: String,
    },
}

fn ask<T: Serialize>(question: impl FnOnce(&Analysis) -> Option<T>) -> Option<String> {
    ANALYSIS.with(|slot| {
        let answer = question(slot.borrow().as_ref()?)?;
        serde_json::to_string(&answer).ok()
    })
}

fn at(file: &str) -> PathBuf {
    PathBuf::from(format!("/{file}"))
}

/// The cart path behind a uri the analysis made from one of [`at`]'s.
fn file_of(uri: &Uri) -> Option<String> {
    let path = lsp::analysis::path_of(uri)?;
    Some(path.to_string_lossy().trim_start_matches('/').to_owned())
}
