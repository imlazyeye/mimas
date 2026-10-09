use crate::{HEIGHT, SHEET_SIZE, SPRITE_SIZE, WIDTH, font};

/// The screen and the sprite sheet, with everything that draws on them. A pixel is a palette
/// index, and the framebuffer keeps its pixels from one frame to the next. Every primitive applies
/// the camera, then clips, so nothing a script passes in can write outside the screen.
pub struct Screen {
    /// One palette index per pixel, row by row.
    pub pixels: [u8; WIDTH * HEIGHT],
    /// The sprite sheet, one palette index per pixel, row by row. Sprite `n` is the 8 by 8 square
    /// at `((n % 16) * 8, (n / 16) * 8)`.
    pub sheet: [u8; SHEET_SIZE * SHEET_SIZE],
    /// Subtracted from the position of everything drawn.
    pub camera: (i64, i64),
    /// Where drawing is allowed, in screen pixels after the camera.
    pub clip: Clip,
    /// The palette index each index draws as.
    pub remap: [u8; 16],
    /// The index sprites and blits skip, or `None` to draw every pixel.
    pub transparent: Option<u8>,
}

/// The part of the screen drawing is allowed in. The right and bottom edges are exclusive, and a
/// clip that holds no pixels has them at or before the left and top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clip {
    pub left: i64,
    pub top: i64,
    pub right: i64,
    pub bottom: i64,
}

impl Screen {
    /// Writes the screen into `out` as RGBA, four bytes a pixel, through [`PALETTE`]. `out` is
    /// `WIDTH * HEIGHT * 4` bytes long.
    pub fn write_rgba(&self, out: &mut [u8]) {
        let (rgba, _) = out.as_chunks_mut::<4>();
        for (index, rgba) in self.pixels.iter().zip(rgba) {
            let [r, g, b] = PALETTE[(index & 15) as usize];
            *rgba = [r, g, b, 255];
        }
    }

    /// Camera, clip, remap and transparency back to what a fresh screen has.
    pub fn reset_state(&mut self) {
        self.camera = (0, 0);
        self.clip = Clip::FULL;
        self.reset_remap();
        self.transparent = Some(0);
    }

    /// Everything drawn from here on is moved by `(-x, -y)`.
    pub fn set_camera(&mut self, x: i64, y: i64) {
        self.camera = (x.clamp(-REACH, REACH), y.clamp(-REACH, REACH));
    }

    /// Limits drawing to the rectangle, cut down to the screen. A size that isn't positive leaves
    /// nothing to draw on.
    pub fn set_clip(&mut self, x: i64, y: i64, w: i64, h: i64) {
        let left = x.clamp(0, WIDTH as i64);
        let top = y.clamp(0, HEIGHT as i64);
        self.clip = Clip {
            left,
            top,
            right: x.saturating_add(w).clamp(left, WIDTH as i64),
            bottom: y.saturating_add(h).clamp(top, HEIGHT as i64),
        };
    }

    /// Removes the clip, so drawing can touch the whole screen again.
    pub fn unclip(&mut self) {
        self.clip = Clip::FULL;
    }

    /// Draws `from` as `to` from here on. A color is taken modulo 16.
    pub fn set_remap(&mut self, from: u8, to: u8) {
        self.remap[(from & 15) as usize] = to & 15;
    }

    /// Draws every color as itself again.
    pub fn reset_remap(&mut self) {
        self.remap = std::array::from_fn(|index| index as u8);
    }

    /// Chooses the color sprites and blits skip, or none. A color is taken modulo 16.
    pub fn set_transparent(&mut self, color: Option<u8>) {
        self.transparent = color.map(|color| color & 15);
    }

    /// The sheet's pixel, or black off the sheet.
    pub fn sheet_pixel(&self, x: i64, y: i64) -> u8 {
        match Self::sheet_index(x, y) {
            Some(index) => self.sheet[index],
            None => 0,
        }
    }

    /// Writes a pixel of the sheet, or nothing off the sheet.
    pub fn set_sheet_pixel(&mut self, x: i64, y: i64, color: u8) {
        if let Some(index) = Self::sheet_index(x, y) {
            self.sheet[index] = color & 15;
        }
    }

    /// The screen's pixel, or black off the screen. The camera applies and the clip doesn't.
    pub fn pixel_at(&self, x: i64, y: i64) -> u8 {
        let (x, y) = self.at(x, y);
        if (0..WIDTH as i64).contains(&x) && (0..HEIGHT as i64).contains(&y) {
            self.pixels[y as usize * WIDTH + x as usize]
        } else {
            0
        }
    }

    /// Fills the whole screen, whatever the camera and clip are.
    pub fn clear(&mut self, color: u8) {
        self.pixels.fill(self.remap[(color & 15) as usize]);
    }

    /// Draws `s` with its top left corner at the position, one glyph every four pixels. A `\n`
    /// starts a line six pixels down, and a character the font doesn't have is a box. Returns
    /// the width of the widest line, which is every glyph's four pixels whether it was on the
    /// screen or not.
    pub fn text(&mut self, s: &str, x: i64, y: i64, color: u8) -> i64 {
        let (left, top) = self.at(x, y);
        let clip = self.clip;
        for (n, line) in s.split('\n').enumerate() {
            let row = top.saturating_add(n as i64 * font::CELL_H);
            if row >= clip.bottom {
                break;
            }
            if row + font::CELL_H <= clip.top {
                continue;
            }
            // glyphs left of the clip aren't worth a look
            let skip = (clip.left - left).max(0) / font::CELL_W;
            for (i, c) in line
                .chars()
                .enumerate()
                .skip(usize::try_from(skip).unwrap_or(usize::MAX))
            {
                let cell = left + i as i64 * font::CELL_W;
                if cell >= clip.right {
                    break;
                }
                for (dy, bits) in font::glyph(c).into_iter().enumerate() {
                    for dx in 0..3 {
                        if bits >> (2 - dx) & 1 == 1 {
                            self.plot(cell + dx, row + dy as i64, color);
                        }
                    }
                }
            }
        }
        Self::text_width(s)
    }

    /// What [`text`](Self::text) returns for `s`: four pixels for each character of its widest
    /// line.
    pub fn text_width(s: &str) -> i64 {
        let widest = s.split('\n').map(|line| line.chars().count()).max();
        widest.unwrap_or(0) as i64 * font::CELL_W
    }

    /// Draws sprite `n` with its top left corner at the position, `scale` times as big, skipping
    /// the transparent color. A sprite number off the sheet draws nothing.
    pub fn sprite(&mut self, n: i64, x: i64, y: i64, flip_x: bool, flip_y: bool, scale: f64) {
        let size = SPRITE_SIZE as i64;
        let per_row = SHEET_SIZE as i64 / size;
        if (0..per_row * per_row).contains(&n) {
            let source = [n % per_row * size, n / per_row * size, size, size];
            self.region(source, x, y, (flip_x, flip_y), scale);
        }
    }

    /// Draws the `[sx, sy, w, h]` part of the sheet with its top left corner at the position, the
    /// way [`sprite`](Self::sprite) does. Whatever part of it is off the sheet draws nothing.
    pub fn blit(&mut self, source: [i64; 4], x: i64, y: i64, scale: f64) {
        self.region(source, x, y, (false, false), scale);
    }

    /// The circle's outline, one pixel thick, around the position. A negative radius draws
    /// nothing, and a radius of zero is a single pixel.
    pub fn circle(&mut self, x: i64, y: i64, r: i64, color: u8) {
        let (cx, cy) = self.at(x, y);
        if r == 0 {
            self.plot(cx, cy, color);
        } else if r > 0 {
            let r = r.min(REACH);
            let (top, bottom) = (
                (cy - r).max(self.clip.top),
                (cy + r + 1).min(self.clip.bottom),
            );
            for row in top..bottom {
                let dy = (row - cy) as i128;
                let outer = ((r as i128).pow(2) - dy * dy).isqrt() as i64;
                // the ring is what's left of the disc after the next circle in
                let inner = (r as i128 - 1).pow(2) - dy * dy;
                if inner < 0 {
                    self.span(row, cx - outer, cx + outer + 1, color);
                } else {
                    let inner = inner.isqrt() as i64;
                    self.span(row, cx - outer, cx - inner, color);
                    self.span(row, cx + inner + 1, cx + outer + 1, color);
                }
            }
        }
    }

    /// The disc around the position, whose pixels are the ones no farther from it than `r`. A
    /// negative radius draws nothing.
    pub fn fill_circle(&mut self, x: i64, y: i64, r: i64, color: u8) {
        let (cx, cy) = self.at(x, y);
        if r >= 0 {
            let r = r.min(REACH);
            let (top, bottom) = (
                (cy - r).max(self.clip.top),
                (cy + r + 1).min(self.clip.bottom),
            );
            for row in top..bottom {
                let dy = (row - cy) as i128;
                let half = ((r as i128).pow(2) - dy * dy).isqrt() as i64;
                self.span(row, cx - half, cx + half + 1, color);
            }
        }
    }

    /// The outline of the `w` by `h` rectangle with its top left corner at the position. A size
    /// that isn't positive draws nothing.
    pub fn rect(&mut self, x: i64, y: i64, w: i64, h: i64, color: u8) {
        let (x, y) = self.at(x, y);
        if w > 0 && h > 0 {
            let (right, bottom) = (x.saturating_add(w), y.saturating_add(h));
            self.span(y, x, right, color);
            if h > 1 {
                self.span(bottom - 1, x, right, color);
            }
            for row in (y + 1).max(self.clip.top)..(bottom - 1).min(self.clip.bottom) {
                self.plot(x, row, color);
                if w > 1 {
                    self.plot(right - 1, row, color);
                }
            }
        }
    }

    /// The `w` by `h` rectangle with its top left corner at the position. A size that isn't
    /// positive draws nothing.
    pub fn fill_rect(&mut self, x: i64, y: i64, w: i64, h: i64, color: u8) {
        let (x, y) = self.at(x, y);
        for row in y.max(self.clip.top)..y.saturating_add(h).min(self.clip.bottom) {
            self.span(row, x, x.saturating_add(w), color);
        }
    }

    /// The line between the two positions, both ends included. It looks the same drawn from
    /// either end.
    pub fn line(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, color: u8) {
        let (x0, y0) = self.at(x0, y0);
        let (x1, y1) = self.at(x1, y1);
        // walk the longer axis, one pixel at a time, and only through the clip
        let steep = (y1 - y0).abs() > (x1 - x0).abs();
        let (a0, b0, a1, b1) = if steep {
            (y0, x0, y1, x1)
        } else {
            (x0, y0, x1, y1)
        };
        let ((a0, b0), (a1, b1)) = if a0 <= a1 {
            ((a0, b0), (a1, b1))
        } else {
            ((a1, b1), (a0, b0))
        };
        let (first, last) = if steep {
            (self.clip.top, self.clip.bottom - 1)
        } else {
            (self.clip.left, self.clip.right - 1)
        };
        let (len, rise) = ((a1 - a0) as i128, (b1 - b0) as i128);
        for a in a0.max(first)..=a1.min(last) {
            let b = match len {
                0 => b0,
                _ => b0 + ((a - a0) as i128 * rise * 2 + len).div_euclid(2 * len) as i64,
            };
            if steep {
                self.plot(b, a, color);
            } else {
                self.plot(a, b, color);
            }
        }
    }

    /// Draws one pixel.
    pub fn pixel(&mut self, x: i64, y: i64, color: u8) {
        let (x, y) = self.at(x, y);
        self.plot(x, y, color);
    }

    /// Draws the `[sx, sy, w, h]` part of the sheet at the position, `scale` times as big and
    /// flipped inside its own rectangle, skipping the pixels that match the transparent color.
    /// Every pixel it covers takes the sheet pixel it falls on, so a scale that isn't a whole
    /// number draws the sheet's pixels in uneven sizes.
    fn region(
        &mut self,
        source: [i64; 4],
        x: i64,
        y: i64,
        (flip_x, flip_y): (bool, bool),
        scale: f64,
    ) {
        let [sx, sy, w, h] = source;
        let (x, y) = self.at(x, y);
        // nothing is drawn unless it comes out at least a pixel each way, which also turns away a
        // scale that isn't a number
        let (across, down) = ((w as f64 * scale).round(), (h as f64 * scale).round());
        if !(w > 0 && h > 0 && across >= 1.0 && down >= 1.0) {
            return;
        }
        // the ranges are worked out as i128 (the rectangle can be as far out as an int goes)
        let clip = self.clip;
        let (left, top) = (i128::from(x), i128::from(y));
        let (right, bottom) = (
            left.saturating_add(across as i128),
            top.saturating_add(down as i128),
        );
        let (dx0, dx1) = (left.max(clip.left.into()), right.min(clip.right.into()));
        let (dy0, dy1) = (top.max(clip.top.into()), bottom.min(clip.bottom.into()));
        if dx0 >= dx1 || dy0 >= dy1 {
            return;
        }
        // all of it is small now
        let offset = |d: i64, size: i64, flip: bool| {
            let offset = ((d as f64 / scale) as i64).min(size - 1);
            if flip { size - 1 - offset } else { offset }
        };
        for dy in dy0 as i64..dy1 as i64 {
            let oy = offset(dy - y, h, flip_y);
            for dx in dx0 as i64..dx1 as i64 {
                let ox = offset(dx - x, w, flip_x);
                let Some(index) = Self::sheet_index(sx.saturating_add(ox), sy.saturating_add(oy))
                else {
                    continue;
                };
                let color = self.sheet[index];
                if self.transparent != Some(color) {
                    self.pixels[dy as usize * WIDTH + dx as usize] =
                        self.remap[color as usize & 15];
                }
            }
        }
    }

    /// Fills `x0..x1` of a row, inside the clip.
    fn span(&mut self, y: i64, x0: i64, x1: i64, color: u8) {
        let Clip {
            left,
            top,
            right,
            bottom,
        } = self.clip;
        let (x0, x1) = (x0.max(left), x1.min(right));
        if y < top || y >= bottom || x1 <= x0 {
            return;
        }
        let color = self.remap[(color & 15) as usize];
        let row = y as usize * WIDTH;
        self.pixels[row + x0 as usize..row + x1 as usize].fill(color);
    }

    /// Writes one pixel if it's inside the clip.
    fn plot(&mut self, x: i64, y: i64, color: u8) {
        let Clip {
            left,
            top,
            right,
            bottom,
        } = self.clip;
        if x < left || x >= right || y < top || y >= bottom {
            return;
        }
        self.pixels[y as usize * WIDTH + x as usize] = self.remap[(color & 15) as usize];
    }

    /// Where a position lands on the screen. It's kept within `REACH`, which is far past the
    /// screen but leaves room for the arithmetic every primitive does with it.
    fn at(&self, x: i64, y: i64) -> (i64, i64) {
        (
            x.saturating_sub(self.camera.0).clamp(-REACH, REACH),
            y.saturating_sub(self.camera.1).clamp(-REACH, REACH),
        )
    }

    fn sheet_index(x: i64, y: i64) -> Option<usize> {
        let side = 0..SHEET_SIZE as i64;
        (side.contains(&x) && side.contains(&y)).then(|| y as usize * SHEET_SIZE + x as usize)
    }
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            pixels: [0; WIDTH * HEIGHT],
            sheet: [0; SHEET_SIZE * SHEET_SIZE],
            camera: (0, 0),
            clip: Clip::FULL,
            remap: std::array::from_fn(|index| index as u8),
            transparent: Some(0),
        }
    }
}

impl Clip {
    pub const FULL: Self = Self {
        left: 0,
        top: 0,
        right: WIDTH as i64,
        bottom: HEIGHT as i64,
    };
}

/// How far from the origin a position can be, so the products a line and a circle make from
/// positions fit in an i128 with room to spare.
const REACH: i64 = 1 << 60;

/// Sweetie 16 by GrafxKid, in the order of the `Color` enum. See
/// <https://lospec.com/palette-list/sweetie-16>.
pub const PALETTE: [[u8; 3]; 16] = [
    [0x1a, 0x1c, 0x2c],
    [0x5d, 0x27, 0x5d],
    [0xb1, 0x3e, 0x53],
    [0xef, 0x7d, 0x57],
    [0xff, 0xcd, 0x75],
    [0xa7, 0xf0, 0x70],
    [0x38, 0xb7, 0x64],
    [0x25, 0x71, 0x79],
    [0x29, 0x36, 0x6f],
    [0x3b, 0x5d, 0xc9],
    [0x41, 0xa6, 0xf6],
    [0x73, 0xef, 0xf7],
    [0xf4, 0xf4, 0xf4],
    [0x94, 0xb0, 0xc2],
    [0x56, 0x6c, 0x86],
    [0x33, 0x3c, 0x57],
];
