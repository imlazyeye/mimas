/// The controls for one frame, as the front end reads them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    /// A bit for each [`Button`], in the order of its variants (see [`Button::bit`]).
    pub held: u16,
    /// Buttons pressed between frames, including taps no longer in [`Self::held`]. Hosts that
    /// only sample held buttons can leave this at zero; the machine also infers transitions.
    pub pressed: u16,
    /// Buttons released between frames, in the same bits as [`Self::held`].
    pub released: u16,
    /// The mouse in screen pixels, or `None` when it's outside the screen.
    pub mouse: Option<(i32, i32)>,
    /// A bit for each [`Mouse`](crate::Mouse) button (see [`Mouse::bit`](crate::Mouse::bit)).
    pub mouse_held: u8,
    /// Mouse buttons pressed between frames, including clicks no longer held.
    pub mouse_pressed: u8,
}
