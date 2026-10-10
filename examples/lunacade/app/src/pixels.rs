//! File transport for the pixel editor cart. The cart uses only the ordinary console API;
//! its live sprite sheet is the document. Opening a file doesn't rewrite it, even if malformed.
use lunacade_core::{Cart, SHEET_SIZE};

pub(crate) type Sheet = [u8; SHEET_SIZE * SHEET_SIZE];

pub(crate) fn cart(text: &str) -> (Cart, Sheet, bool) {
    let mut sheet = [0; SHEET_SIZE * SHEET_SIZE];
    let mut clean = true;
    for (y, raw) in text.split_terminator('\n').enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if y >= SHEET_SIZE {
            clean &= line.is_empty();
            continue;
        }
        for (x, c) in line.chars().enumerate() {
            match (x < SHEET_SIZE, c.to_digit(16)) {
                (true, Some(color)) => sheet[y * SHEET_SIZE + x] = color as u8,
                _ => clean = false,
            }
        }
    }
    let cart = Cart {
        files: [
            (
                "pixels.mim".to_owned(),
                include_str!("../../editors/pixels/pixels.mim").to_owned(),
            ),
            ("sprites.txt".to_owned(), serialize(&sheet)),
        ]
        .into(),
    };
    (cart, sheet, clean)
}

/// Omit trailing black pixels and rows, as the cart format fills them back in with black.
pub(crate) fn serialize(sheet: &Sheet) -> String {
    let mut text = String::new();
    let rows: Vec<_> = sheet.chunks(SHEET_SIZE).collect();
    let end = rows.iter().rposition(|row| row.iter().any(|&c| c != 0));
    for row in rows.iter().take(end.map_or(0, |y| y + 1)) {
        let end = row.iter().rposition(|&c| c != 0);
        for &color in row.iter().take(end.map_or(0, |x| x + 1)) {
            text.push(b"0123456789abcdef"[color as usize] as char);
        }
        text.push('\n');
    }
    text
}
