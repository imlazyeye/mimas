use std::path::Path;

use lunacade_core::{Button, Cart, Input, Machine, Mouse};

fn editor() -> Machine {
    let cart = Cart::from_dir(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../editors/pixels"
    )))
    .unwrap();
    Machine::load(&cart, 0).unwrap_or_else(|problems| panic!("{problems:#?}"))
}

fn frame(machine: &mut Machine, input: Input) {
    assert!(!machine.frame(&input));
    assert_eq!(machine.fault(), None);
}

fn pointer(machine: &mut Machine, x: i32, y: i32, held: u8, pressed: u8) {
    frame(
        machine,
        Input {
            mouse: Some((x, y)),
            mouse_held: held,
            mouse_pressed: pressed,
            ..Input::default()
        },
    );
}

fn tap(machine: &mut Machine, x: i32, y: i32) {
    pointer(machine, x, y, 0, Mouse::Left.bit());
}

fn key(machine: &mut Machine, button: Button) {
    frame(
        machine,
        Input {
            pressed: button.bit(),
            ..Input::default()
        },
    );
}

#[test]
fn draw_erase_and_travel_by_whole_stroke() {
    let mut m = editor();
    frame(&mut m, Input::default());
    pointer(&mut m, 149, 9, Mouse::Left.bit(), Mouse::Left.bit());
    pointer(&mut m, 233, 93, Mouse::Left.bit(), 0);
    pointer(&mut m, 233, 93, 0, 0);
    for i in 0..8 {
        assert_eq!(m.screen().sheet[i * 128 + i], 12);
    }
    key(&mut m, Button::A);
    assert!(m.screen().sheet.iter().all(|&c| c == 0));
    key(&mut m, Button::B);
    for i in 0..8 {
        assert_eq!(m.screen().sheet[i * 128 + i], 12);
    }
    pointer(&mut m, 149, 9, 0, Mouse::Right.bit());
    assert_eq!(m.screen().sheet[0], 0);
    key(&mut m, Button::A);
    assert_eq!(m.screen().sheet[0], 12);
}

#[test]
fn select_palette_and_last_sprite_and_undo_across_sprites() {
    let mut m = editor();
    tap(&mut m, 130, 134); // sprite 255
    tap(&mut m, 185, 109); // orange
    tap(&mut m, 149, 9);
    assert_eq!(m.screen().sheet[120 * 128 + 120], 3);
    key(&mut m, Button::Right); // clamp, don't wrap
    key(&mut m, Button::Down);
    tap(&mut m, 161, 9);
    assert_eq!(m.screen().sheet[120 * 128 + 121], 3);
    tap(&mut m, 5, 9); // sprite 0
    tap(&mut m, 149, 9);
    key(&mut m, Button::A);
    assert_eq!(m.screen().sheet[0], 0);
    key(&mut m, Button::A);
    assert_eq!(m.screen().sheet[120 * 128 + 121], 0);
    key(&mut m, Button::B);
    assert_eq!(m.screen().sheet[120 * 128 + 121], 3);
}

#[test]
fn no_op_stroke_preserves_redo_and_new_edit_clears_it() {
    let mut m = editor();
    tap(&mut m, 149, 9);
    key(&mut m, Button::A);
    pointer(&mut m, 149, 9, 0, Mouse::Right.bit()); // already black
    key(&mut m, Button::B);
    assert_eq!(m.screen().sheet[0], 12);
    key(&mut m, Button::A);
    tap(&mut m, 161, 9);
    key(&mut m, Button::B);
    assert_eq!(m.screen().sheet[0], 0);
    assert_eq!(m.screen().sheet[1], 12);
}

#[test]
fn drag_clamps_to_sprite_and_release_off_screen_finishes_history() {
    let mut m = editor();
    pointer(&mut m, 149, 9, Mouse::Left.bit(), Mouse::Left.bit());
    pointer(&mut m, 255, 9, Mouse::Left.bit(), 0);
    frame(&mut m, Input::default());
    assert!(m.screen().sheet[..8].iter().all(|&c| c == 12));
    assert!(m.screen().sheet[8..].iter().all(|&c| c == 0));
    tap(&mut m, 155, 134); // undo button
    assert!(m.screen().sheet.iter().all(|&c| c == 0));
    tap(&mut m, 205, 134); // redo button
    assert!(m.screen().sheet[..8].iter().all(|&c| c == 12));
}

#[test]
fn all_palette_colors_and_release_with_undo() {
    let mut m = editor();
    for color in 0..16 {
        tap(&mut m, 149 + color % 8 * 12, 109 + color / 8 * 9);
        tap(&mut m, 149 + color % 8 * 12, 9 + color / 8 * 12);
        assert_eq!(
            m.screen().sheet[(color / 8 * 128 + color % 8) as usize],
            color as u8
        );
    }
    pointer(&mut m, 149, 93, Mouse::Left.bit(), Mouse::Left.bit());
    assert_eq!(m.screen().sheet[7 * 128], 15);
    // The release and shortcut may both arrive between frames.
    key(&mut m, Button::A);
    assert_eq!(m.screen().sheet[7 * 128], 0);
}

#[test]
fn history_is_bounded() {
    let mut m = editor();
    for i in 0..51 {
        tap(&mut m, 149 + (i % 8) * 12, 9 + (i / 8) * 12);
    }
    for _ in 0..51 {
        key(&mut m, Button::A);
    }
    assert_eq!(m.screen().sheet[0], 12);
    assert_eq!(m.screen().sheet.iter().filter(|&&c| c != 0).count(), 1);
}
