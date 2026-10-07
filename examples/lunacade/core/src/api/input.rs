use std::cell::Cell;

use mimas::{
    Ctx, MimasEnum, native,
    vm::{api::Api, glam::I64Vec2},
};

use crate::{HEIGHT, WIDTH, input::Input};

/// A button on the console's controller. The d-pad and the four face buttons come from a keyboard
/// or a gamepad.
#[derive(MimasEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    /// Up on the d-pad. On a keyboard it's the up arrow or W.
    Up,
    /// Down on the d-pad. On a keyboard it's the down arrow or S.
    Down,
    /// Left on the d-pad. On a keyboard it's the left arrow or A.
    Left,
    /// Right on the d-pad. On a keyboard it's the right arrow or D.
    Right,
    /// The A button. On a keyboard it's Z or J.
    A,
    /// The B button. On a keyboard it's X or K.
    B,
    /// The X button. On a keyboard it's C or L.
    X,
    /// The Y button. On a keyboard it's V or `;`.
    Y,
    /// The Start button. On a keyboard it's Enter.
    Start,
}

/// A button on the mouse.
#[derive(MimasEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mouse {
    /// The left mouse button.
    Left,
    /// The right mouse button.
    Right,
}

/// The controls as the `input` natives see them, kept up to date by the machine. A press or a
/// release stays here until a frame runs, so one that lands on a skipped frame isn't lost.
#[derive(Default)]
pub(crate) struct Controls {
    held: Cell<u16>,
    pressed: Cell<u16>,
    released: Cell<u16>,
    mouse: Cell<Option<(i32, i32)>>,
    mouse_moved: Cell<bool>,
    mouse_held: Cell<u8>,
    mouse_pressed: Cell<u8>,
}

impl Button {
    /// Every button, in the order of its variants.
    pub const ALL: [Button; 9] = [
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::A,
        Self::B,
        Self::X,
        Self::Y,
        Self::Start,
    ];

    /// The bit this button is in [`Input::held`].
    pub fn bit(self) -> u16 {
        1 << self as u16
    }
}

impl Mouse {
    /// The bit this button is in [`Input::mouse_held`].
    pub fn bit(self) -> u8 {
        1 << self as u8
    }
}

impl Controls {
    /// Takes in the input of a frame that comes after `previous`, and remembers what was pressed
    /// and released between the two.
    pub fn feed(&self, previous: &Input, input: &Input) {
        self.held.set(input.held);
        self.pressed
            .set(self.pressed.get() | input.pressed | input.held & !previous.held);
        self.released
            .set(self.released.get() | input.released | !input.held & previous.held);
        self.mouse.set(input.mouse);
        self.mouse_moved
            .set(self.mouse_moved.get() || input.mouse != previous.mouse);
        self.mouse_held.set(input.mouse_held);
        self.mouse_pressed.set(
            self.mouse_pressed.get()
                | input.mouse_pressed
                | input.mouse_held & !previous.mouse_held,
        );
    }

    /// Forgets the presses and releases, once a frame has had its look at them.
    pub fn clear_edges(&self) {
        self.pressed.set(0);
        self.released.set(0);
        self.mouse_moved.set(false);
        self.mouse_pressed.set(0);
    }
}

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    let mut input = api.module("input");
    input.add(held);
    input.add(pressed);
    input.add(released);
    input.add(mouse);
    input.add(mouse_moved);
    input.add(mouse_held);
    input.add(mouse_pressed);
}

/// Returns whether the button is down right now.
///
/// ```mimas
/// let speed = if input::held(Button::B) 3 else 1;
/// ```
#[native]
fn held<'gc>(ctx: Ctx<'gc>, b: Button) -> bool {
    ctx.fixture::<Controls>().held.get() & b.bit() != 0
}

/// Returns whether the button went down since the last frame ran. A press that happens while the
/// console is skipping frames still counts on the next frame that runs.
///
/// ```mimas
/// if input::pressed(Button::A) {
///     print("jump");
/// }
/// ```
#[native]
fn pressed<'gc>(ctx: Ctx<'gc>, b: Button) -> bool {
    ctx.fixture::<Controls>().pressed.get() & b.bit() != 0
}

/// Returns whether the button came up since the last frame ran, with the same catch-up as
/// [`pressed`](#pressed).
///
/// ```mimas
/// if input::released(Button::A) {
///     print("let go");
/// }
/// ```
#[native]
fn released<'gc>(ctx: Ctx<'gc>, b: Button) -> bool {
    ctx.fixture::<Controls>().released.get() & b.bit() != 0
}

/// Returns the mouse's position in screen pixels, or `null` when it's outside the screen.
///
/// ```mimas
/// if let pos? = input::mouse() {
///     gfx::pixel(pos, Color::White);
/// }
/// ```
#[native]
fn mouse<'gc>(ctx: Ctx<'gc>) -> Option<I64Vec2> {
    let (x, y) = ctx.fixture::<Controls>().mouse.get()?;
    let on_screen = (0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y);
    on_screen.then_some(I64Vec2::new(x.into(), y.into()))
}

/// Returns whether the mouse is somewhere else than it was when the last frame ran. A cart that
/// also takes the d-pad can check this so that a mouse lying still doesn't fight it.
///
/// ```mimas
/// if let pos? = input::mouse() {
///     if input::mouse_moved() {
///         print(f"now at {pos.x}, {pos.y}");
///     }
/// }
/// ```
#[native]
fn mouse_moved<'gc>(ctx: Ctx<'gc>) -> bool {
    ctx.fixture::<Controls>().mouse_moved.get()
}

/// Returns whether the mouse button is down right now.
///
/// ```mimas
/// let drawing = input::mouse_held(Mouse::Left);
/// ```
#[native]
fn mouse_held<'gc>(ctx: Ctx<'gc>, m: Mouse) -> bool {
    ctx.fixture::<Controls>().mouse_held.get() & m.bit() != 0
}

/// Returns whether the mouse button went down since the last frame ran, with the same catch-up as
/// [`pressed`](#pressed).
///
/// ```mimas
/// if input::mouse_pressed(Mouse::Left) {
///     print("click");
/// }
/// ```
#[native]
fn mouse_pressed<'gc>(ctx: Ctx<'gc>, m: Mouse) -> bool {
    ctx.fixture::<Controls>().mouse_pressed.get() & m.bit() != 0
}
