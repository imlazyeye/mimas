use lunacade::Console;
use lunacade_core::{Cart, Color};

fn draw(editor: &mut Console, x: i32, y: i32, right: bool) {
    assert!(editor.frame(0, 0, 0, x, y, 0, if right { 2 } else { 1 }));
    assert_eq!(editor.fault(), None);
}

fn file(text: &str) -> Cart {
    Cart {
        files: [
            ("main.mim".to_owned(), "luna::draw(|| {});".to_owned()),
            ("sprites.txt".to_owned(), text.to_owned()),
        ]
        .into(),
    }
}

#[test]
fn opening_and_selecting_never_rewrites_the_file() {
    for text in [
        "",
        "C0\r\n\n",
        "not hex!",
        &"f".repeat(129),
        &"\n".repeat(130),
    ] {
        let mut editor = Console::pixel_editor(text).unwrap();
        assert_eq!(editor.take_sheet(), None);
        draw(&mut editor, 20, 20, false); // select a sprite
        draw(&mut editor, 173, 110, false); // select a color
        assert_eq!(editor.take_sheet(), None);
        assert_eq!(editor.sheet_is_clean(), file(text).validate().is_ok());
    }
}

#[test]
fn changed_sheet_round_trips_and_only_exports_once() {
    let mut editor = Console::pixel_editor("A\r\n").unwrap();
    draw(&mut editor, 161, 9, false); // draw white at (1,0)
    assert_eq!(editor.take_sheet().as_deref(), Some("ac\n"));
    assert_eq!(editor.take_sheet(), None);
    assert!(editor.frame(0, 1 << 4, 0, -1, -1, 0, 0)); // undo
    assert_eq!(editor.take_sheet().as_deref(), Some("a\n"));
    draw(&mut editor, 149, 9, true);
    assert_eq!(editor.take_sheet().as_deref(), Some(""));
    draw(&mut editor, 130, 134, false); // last sprite
    draw(&mut editor, 233, 93, false); // last pixel in the sheet
    let text = editor.take_sheet().unwrap();
    let cart = file(&text);
    let sheet = cart.validate().unwrap().sheet;
    assert_eq!(sheet[128 * 128 - 1], Color::White as u8);
    assert_eq!(sheet.iter().filter(|&&c| c != 0).count(), 1);
    let mut reloaded = Console::pixel_editor(&text).unwrap();
    assert!(reloaded.sheet_is_clean());
    assert_eq!(reloaded.take_sheet(), None);
    // A new file starts a new history.
    assert!(reloaded.frame(0, 1 << 4, 0, -1, -1, 0, 0));
    assert_eq!(reloaded.take_sheet(), None);
}

#[test]
fn malformed_text_is_normalized_only_after_a_real_edit() {
    let mut editor = Console::pixel_editor("F!2\n").unwrap();
    assert!(!editor.sheet_is_clean());
    draw(&mut editor, 161, 9, true); // erasing an already-black invalid cell is not an edit
    assert_eq!(editor.take_sheet(), None);
    assert!(!editor.sheet_is_clean());
    draw(&mut editor, 161, 9, false);
    let text = editor.take_sheet().unwrap();
    assert_eq!(text, "fc2\n");
    assert!(editor.sheet_is_clean());
    assert!(file(&text).validate().is_ok());
}

#[test]
fn game_sheet_mutations_never_export_an_editor_document() {
    let cart = serde_json::json!({ "files": { "game.mim":
        "luna::draw(|| gfx::set_sheet_pixel(std::math::ivec2(0, 0), Color::Red));"
    }});
    let mut game = Console::new(&cart.to_string(), 0.0).unwrap();
    assert!(game.frame(0, 0, 0, -1, -1, 0, 0));
    assert_eq!(game.fault(), None);
    assert_eq!(game.take_sheet(), None);
}
