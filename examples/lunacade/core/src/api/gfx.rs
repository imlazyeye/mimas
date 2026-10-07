use std::cell::{RefCell, RefMut};

use mimas::{
    Ctx, MimasEnum, native,
    vm::{api::Api, glam::I64Vec2},
};

use crate::{Screen, WIDTH};

/// One of the 16 colors of the Sweetie 16 palette by GrafxKid, which every cart draws with (see
/// <https://lospec.com/palette-list/sweetie-16>). The variants are in palette order, so
/// `Color::Black` is `0` and `Color::Charcoal` is `15`.
///
/// ```mimas
/// gfx::clear(Color::Navy);
/// let next = Color::from_int(Color::Red.to_int() + 1); // Color::Orange
/// ```
#[derive(MimasEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    /// Color 0, `#1a1c2c`.
    Black,
    /// Color 1, `#5d275d`.
    Plum,
    /// Color 2, `#b13e53`.
    Red,
    /// Color 3, `#ef7d57`.
    Orange,
    /// Color 4, `#ffcd75`.
    Yellow,
    /// Color 5, `#a7f070`.
    Lime,
    /// Color 6, `#38b764`.
    Green,
    /// Color 7, `#257179`.
    Teal,
    /// Color 8, `#29366f`.
    Navy,
    /// Color 9, `#3b5dc9`.
    Blue,
    /// Color 10, `#41a6f6`.
    Sky,
    /// Color 11, `#73eff7`.
    Cyan,
    /// Color 12, `#f4f4f4`.
    White,
    /// Color 13, `#94b0c2`.
    Silver,
    /// Color 14, `#566c86`.
    Slate,
    /// Color 15, `#333c57`.
    Charcoal,
}

impl Color {
    /// Every color, in palette order.
    pub const ALL: [Color; 16] = [
        Self::Black,
        Self::Plum,
        Self::Red,
        Self::Orange,
        Self::Yellow,
        Self::Lime,
        Self::Green,
        Self::Teal,
        Self::Navy,
        Self::Blue,
        Self::Sky,
        Self::Cyan,
        Self::White,
        Self::Silver,
        Self::Slate,
        Self::Charcoal,
    ];

    /// The color for a palette index, taken modulo 16.
    pub fn from_index(index: u8) -> Self {
        Self::ALL[(index & 15) as usize]
    }
}

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    api.add_method(to_int);
    api.add_assoc_of::<Color, _, _>("from_int", from_int);

    let mut gfx = api.module("gfx");
    gfx.add(clear);
    gfx.add(pixel);
    gfx.add(pixel_at);
    gfx.add(line);
    gfx.add(rect);
    gfx.add(fill_rect);
    gfx.add(circle);
    gfx.add(fill_circle);
    gfx.add(sprite);
    gfx.add(blit);
    gfx.add(text);
    gfx.add(text_centered);
    gfx.add(text_width);
    gfx.add(camera);
    gfx.add(clip);
    gfx.add(unclip);
    gfx.add(remap);
    gfx.add(reset_remap);
    gfx.add(transparent);
    gfx.add(sheet_pixel);
    gfx.add(set_sheet_pixel);
}

/// Returns the color's place in the palette, from `0` for `Color::Black` to `15` for
/// `Color::Charcoal`.
///
/// ```mimas
/// let n = Color::Red.to_int(); // 2
/// ```
#[native]
fn to_int(c: Color) -> i64 {
    c as i64
}

/// Returns the color at a place in the palette. A number outside of `0` to `15` wraps around, so
/// `16` is `Color::Black` again and `-1` is `Color::Charcoal`.
///
/// ```mimas
/// let a = Color::from_int(4);  // Color::Yellow
/// let b = Color::from_int(18); // Color::Red
/// ```
#[native]
fn from_int(i: i64) -> Color {
    Color::from_index(i.rem_euclid(16) as u8)
}

/// Fills the whole screen with the color. The camera and the clip don't apply.
///
/// Nothing clears the screen on its own, so a draw usually starts with this.
///
/// ```mimas
/// luna::draw(|| gfx::clear(Color::Navy));
/// ```
#[native]
fn clear<'gc>(ctx: Ctx<'gc>, c: Color) {
    screen(ctx).clear(c as u8);
}

/// Draws one pixel. A pixel off the screen, or outside the clip, isn't drawn.
///
/// ```mimas
/// use std::math::ivec2;
///
/// gfx::pixel(ivec2(128, 72), Color::White);
/// ```
#[native]
fn pixel<'gc>(ctx: Ctx<'gc>, pos: I64Vec2, c: Color) {
    screen(ctx).pixel(pos.x, pos.y, c as u8);
}

/// Returns the color of the pixel on the screen. The camera applies, and the clip doesn't. A
/// position off the screen is `Color::Black`.
///
/// ```mimas
/// use std::math::ivec2;
///
/// let pos = ivec2(10, 10);
/// gfx::pixel(pos, Color::Red);
/// let c = gfx::pixel_at(pos); // Color::Red
/// ```
#[native]
fn pixel_at<'gc>(ctx: Ctx<'gc>, pos: I64Vec2) -> Color {
    Color::from_index(screen(ctx).pixel_at(pos.x, pos.y))
}

/// Draws a line one pixel wide from `a` to `b`, both ends included. It looks the same drawn
/// from either end, and the parts of it off the screen aren't drawn.
///
/// ```mimas
/// use std::math::ivec2;
///
/// gfx::line(ivec2(0, 0), ivec2(255, 143), Color::Sky);
/// ```
#[native]
fn line<'gc>(ctx: Ctx<'gc>, a: I64Vec2, b: I64Vec2, c: Color) {
    screen(ctx).line(a.x, a.y, b.x, b.y, c as u8);
}

/// Draws the outline of a rectangle with its top left corner at `pos`, `size.x` wide and `size.y`
/// tall. A width or height that isn't above `0` draws nothing.
///
/// ```mimas
/// use std::math::ivec2;
///
/// gfx::rect(ivec2(8, 8), ivec2(32, 16), Color::White);
/// ```
#[native]
fn rect<'gc>(ctx: Ctx<'gc>, pos: I64Vec2, size: I64Vec2, c: Color) {
    screen(ctx).rect(pos.x, pos.y, size.x, size.y, c as u8);
}

/// Draws a filled rectangle with its top left corner at `pos`, `size.x` wide and `size.y` tall. A
/// width or height that isn't above `0` draws nothing.
///
/// ```mimas
/// use std::math::ivec2;
///
/// gfx::fill_rect(ivec2(8, 8), ivec2(32, 16), Color::Red);
/// ```
#[native]
fn fill_rect<'gc>(ctx: Ctx<'gc>, pos: I64Vec2, size: I64Vec2, c: Color) {
    screen(ctx).fill_rect(pos.x, pos.y, size.x, size.y, c as u8);
}

/// Draws the outline of a circle around `center`, one pixel thick. A radius of `0` is a single
/// pixel, and a negative one draws nothing.
///
/// ```mimas
/// use std::math::ivec2;
///
/// gfx::circle(ivec2(128, 72), 20, Color::Yellow);
/// ```
#[native]
fn circle<'gc>(ctx: Ctx<'gc>, center: I64Vec2, r: i64, c: Color) {
    screen(ctx).circle(center.x, center.y, r, c as u8);
}

/// Draws a filled circle around `center`, covering the pixels no farther from it than `r`. A
/// negative radius draws nothing.
///
/// ```mimas
/// use std::math::ivec2;
///
/// gfx::fill_circle(ivec2(128, 72), 20, Color::Orange);
/// ```
#[native]
fn fill_circle<'gc>(ctx: Ctx<'gc>, center: I64Vec2, r: i64, c: Color) {
    screen(ctx).fill_circle(center.x, center.y, r, c as u8);
}

/// Draws sprite `n` from the sprite sheet with its top left corner at `pos`. The sheet holds
/// 256 sprites of 8 by 8 pixels, numbered across and then down, so sprite `17` is the second one
/// in the second row. A number off the sheet draws nothing.
///
/// Pixels of the [transparent](#transparent) color are skipped. `flip_x` and `flip_y` mirror the
/// sprite, and both can be left out.
///
/// ```mimas
/// use std::math::ivec2;
///
/// gfx::sprite(0, ivec2(100, 60));
/// gfx::sprite(0, ivec2(108, 60), true);
/// ```
#[native]
fn sprite<'gc>(ctx: Ctx<'gc>, n: i64, pos: I64Vec2, flip_x: Option<bool>, flip_y: Option<bool>) {
    let (flip_x, flip_y) = (flip_x.unwrap_or(false), flip_y.unwrap_or(false));
    screen(ctx).sprite(n, pos.x, pos.y, flip_x, flip_y);
}

/// Copies the `size.x` by `size.y` part of the sprite sheet that starts at `src` to the screen,
/// with its top left corner at `pos`. It draws a larger piece of the sheet than a sprite, and skips
/// the [transparent](#transparent) color the same way. Whatever part of it is off the sheet
/// draws nothing.
///
/// ```mimas
/// use std::math::ivec2;
///
/// gfx::blit(ivec2(0, 0), ivec2(32, 16), ivec2(8, 8));
/// ```
#[native]
fn blit<'gc>(ctx: Ctx<'gc>, src: I64Vec2, size: I64Vec2, pos: I64Vec2) {
    screen(ctx).blit(src.x, src.y, size.x, size.y, pos.x, pos.y);
}

/// Draws the text with its top left corner at `pos` and returns how wide it is in pixels. The
/// font is 4 by 6 pixels a character, and a `\n` starts a line 6 pixels down. A character the
/// font doesn't have is drawn as a box.
///
/// ```mimas
/// use std::math::ivec2;
///
/// let w = gfx::text("score 10", ivec2(4, 4), Color::White); // 32
/// ```
#[native]
fn text<'gc>(ctx: Ctx<'gc>, s: &str, pos: I64Vec2, c: Color) -> i64 {
    screen(ctx).text(s, pos.x, pos.y, c as u8)
}

/// Draws the text like [`text`](#text) does, with its top at `y` and its widest line in the middle
/// of the screen.
///
/// ```mimas
/// gfx::text_centered("GAME OVER", 68, Color::Red);
/// ```
#[native]
fn text_centered<'gc>(ctx: Ctx<'gc>, s: &str, y: i64, c: Color) {
    let x = (WIDTH as i64 - Screen::text_width(s)) / 2;
    screen(ctx).text(s, x, y, c as u8);
}

/// Returns how wide [`text`](#text) draws the string, which is 4 pixels for each character of its
/// longest line.
///
/// ```mimas
/// let x = 128 - gfx::text_width("game over") ~/ 2;
/// ```
#[native]
fn text_width<'gc>(s: &str) -> i64 {
    Screen::text_width(s)
}

/// Moves everything drawn after this back by `pos`, which is how a view follows a player in a
/// world bigger than the screen. A shape drawn at `(100, 100)` with the camera at `(90, 90)` lands
/// at `(10, 10)`. [`clear`](#clear) ignores the camera.
///
/// The camera, the clip, the remap and the transparent color all go back to their defaults at the
/// start of every frame.
///
/// ```mimas
/// use std::math::ivec2;
///
/// let player = ivec2(300, 200);
/// gfx::camera(player.sub(ivec2(128, 72)));
/// ```
#[native]
fn camera<'gc>(ctx: Ctx<'gc>, pos: I64Vec2) {
    screen(ctx).set_camera(pos.x, pos.y);
}

/// Limits drawing to the rectangle with its top left corner at `pos`, `size.x` wide and `size.y`
/// tall, cut down to the screen. A width or height that isn't above `0` leaves nothing to draw on.
/// The clip is in screen pixels, so the camera doesn't move it.
///
/// ```mimas
/// use std::math::ivec2;
///
/// gfx::clip(ivec2(0, 16), ivec2(256, 128));
/// ```
#[native]
fn clip<'gc>(ctx: Ctx<'gc>, pos: I64Vec2, size: I64Vec2) {
    screen(ctx).set_clip(pos.x, pos.y, size.x, size.y);
}

/// Removes the rectangle set by [`clip`](#clip), so drawing can touch the whole screen again.
///
/// ```mimas
/// gfx::unclip();
/// ```
#[native]
fn unclip<'gc>(ctx: Ctx<'gc>) {
    screen(ctx).unclip();
}

/// Draws the color `from` as `to` from here on, for shapes, text and sprites alike. Can be used as
/// a hacky way to create "luts" for sprites.
///
/// ```mimas
/// use std::math::ivec2;
///
/// gfx::remap(Color::Red, Color::Blue);
/// gfx::sprite(1, ivec2(40, 40)); // the red in sprite 1 comes out blue
/// gfx::reset_remap();
/// ```
#[native]
fn remap<'gc>(ctx: Ctx<'gc>, from: Color, to: Color) {
    screen(ctx).set_remap(from as u8, to as u8);
}

/// Draws every color as itself again after [`remap`](#remap).
///
/// ```mimas
/// gfx::reset_remap();
/// ```
#[native]
fn reset_remap<'gc>(ctx: Ctx<'gc>) {
    screen(ctx).reset_remap();
}

/// Chooses the color that [`sprite`](#sprite) and [`blit`](#blit) skip, so a sprite doesn't have
/// to be a rectangle. It starts out as `Color::Black`. `null` skips nothing, and draws every
/// pixel.
///
/// ```mimas
/// use std::math::ivec2;
///
/// gfx::transparent(Color::Plum);
/// gfx::sprite(2, ivec2(20, 20));
/// gfx::transparent(Color::Black);
/// ```
#[native]
fn transparent<'gc>(ctx: Ctx<'gc>, c: Option<Color>) {
    screen(ctx).set_transparent(c.map(|c| c as u8));
}

/// Returns the color of a pixel in the sprite sheet, which is 128 by 128 pixels. A position off
/// the sheet is `Color::Black`.
///
/// ```mimas
/// use std::math::ivec2;
///
/// let c = gfx::sheet_pixel(ivec2(3, 4));
/// ```
#[native]
fn sheet_pixel<'gc>(ctx: Ctx<'gc>, pos: I64Vec2) -> Color {
    Color::from_index(screen(ctx).sheet_pixel(pos.x, pos.y))
}

/// Changes a pixel in the sprite sheet, which is how a cart makes sprites while it runs. A
/// position off the sheet is ignored. The change lasts until the cart is loaded again, and it
/// isn't saved to `sprites.txt`.
///
/// ```mimas
/// use std::math::ivec2;
///
/// gfx::set_sheet_pixel(ivec2(3, 4), Color::White);
/// ```
#[native]
fn set_sheet_pixel<'gc>(ctx: Ctx<'gc>, pos: I64Vec2, c: Color) {
    screen(ctx).set_sheet_pixel(pos.x, pos.y, c as u8);
}

fn screen<'gc>(ctx: Ctx<'gc>) -> RefMut<'gc, Screen> {
    ctx.fixture::<RefCell<Screen>>().borrow_mut()
}
