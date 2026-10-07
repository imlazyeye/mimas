/// The controls for one frame, as the front end reads them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    /// A bit for each [`Button`], in the order of its variants (see [`Button::bit`]).
    pub held: u16,
    /// The mouse in screen pixels, or `None` when it's outside the screen.
    pub mouse: Option<(i32, i32)>,
    /// A bit for each [`Mouse`](crate::Mouse) button (see [`Mouse::bit`](crate::Mouse::bit)).
    pub mouse_held: u8,
}
