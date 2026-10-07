mod runner;

use std::{fs, path::Path};

use lunacade_core::{Button, Cart, DiagnosticKind, HEIGHT, Input, Machine, Mouse, WIDTH};
use runner::{machine, problems, run};

fn top(body: &str) -> Vec<String> {
    let main = format!(
        "{body}
         luna::draw(|| {{}});"
    );
    run(&main, 0)
}

fn pixel(machine: &Machine, x: usize, y: usize) -> u8 {
    machine.screen().pixels[y * WIDTH + x]
}

#[test]
fn test_card() {
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/carts/card"));
    let mut machine = Machine::load(&Cart::from_dir(dir).unwrap(), 1).unwrap();
    machine.frame(&Input::default());
    assert_eq!(machine.fault(), None);
    let rows: Vec<String> = machine
        .screen()
        .pixels
        .chunks(WIDTH)
        .map(|row| row.iter().map(|pixel| format!("{pixel:x}")).collect())
        .collect();
    let expected = dir.join("expected.txt");
    // look the screen over before blessing it
    if std::env::var_os("LUNACADE_BLESS").is_some() {
        fs::write(&expected, rows.join("\n") + "\n").unwrap();
    }
    let expected = fs::read_to_string(expected).unwrap();
    assert_eq!(rows.len(), expected.lines().count());
    for (y, (drawn, expected)) in rows.iter().zip(expected.lines()).enumerate() {
        assert_eq!(drawn, expected, "row {y}");
    }
}

#[test]
fn hello_runs() {
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/carts/hello"));
    let mut machine = Machine::load(&Cart::from_dir(dir).unwrap(), 1).unwrap();
    assert_eq!(machine.take_output(), ["hello from the cart"]);
    assert_eq!(machine.tick(), 0);
    for frame in 1..=5 {
        assert!(!machine.frame(&Input::default()));
        let screen = machine.screen();
        assert_eq!(screen.pixels[68 * WIDTH + frame + 2], 4);
        assert_eq!(screen.pixels[68 * WIDTH + frame], 8);
        assert_eq!(screen.pixels[68 * WIDTH + frame - 1], 8);
    }
    assert!(machine.fault().is_none());
}

#[test]
fn color_ints() {
    let lines = top("print(Color::from_int(18) == Color::Red);
         print(Color::from_int(-1).to_int());
         print(Color::from_int(16).to_int());
         print(Color::Navy.to_int());
         print(Color::Charcoal.to_int());");
    assert_eq!(lines, ["true", "15", "0", "8", "15"]);
}

#[test]
fn luna_values() {
    let lines = run(
        "print(luna::WIDTH);
         print(luna::HEIGHT);
         luna::draw(|| {
             print(luna::frame());
         });",
        3,
    );
    assert_eq!(lines, ["256", "144", "0", "1", "2"]);
}

#[test]
fn runaway_update_faults() {
    let mut machine = machine(
        "luna::update(|| loop {});
         luna::draw(|| {});",
    );
    assert!(!machine.frame(&Input::default()));
    let fault = machine.fault().expect("the machine faulted");
    assert_eq!(fault.message, "ran out of fuel");
    assert_eq!(fault.file, "main.mim");
    assert_eq!(fault.kind, DiagnosticKind::Fault);
    assert!(machine.frame(&Input::default()));
    assert_eq!(machine.tick(), 0);
    assert_eq!(pixel(&machine, 0, 0), 8);
}

#[test]
fn runaway_top_level_faults() {
    let found = problems(
        "loop {}
         luna::draw(|| {});",
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].message, "ran out of fuel");
    assert_eq!(found[0].kind, DiagnosticKind::Fault);
}

#[test]
fn hooks_register_at_the_top_level() {
    let mut machine = machine(
        "luna::update(|| luna::draw(|| {}));
         luna::draw(|| {});",
    );
    machine.frame(&Input::default());
    let fault = machine.fault().expect("the machine faulted");
    assert_eq!(fault.message, "`luna::draw` registers in top-level code");
}

#[test]
fn draw_is_required() {
    let found = problems("luna::update(|| {});");
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].message,
        "carts must provide a draw callback (`luna::draw`)"
    );
}

#[test]
fn state_resets_each_frame() {
    let mut machine = machine(
        "use std::math::ivec2;
         gfx::camera(ivec2(10, 20));
         gfx::clip(ivec2(0, 0), ivec2(4, 4));
         gfx::remap(Color::Red, Color::Blue);
         gfx::transparent(null);
         luna::draw(|| {
             gfx::pixel(ivec2(100, 100), Color::Red);
             print(f\"{gfx::pixel_at(ivec2(100, 100)).to_int()}\");
         });",
    );
    machine.frame(&Input::default());
    assert_eq!(machine.take_output(), ["2"]);
    assert_eq!(pixel(&machine, 100, 100), 2);
}

#[test]
fn text_widths() {
    let mut machine = machine(
        "use std::math::ivec2;
         let w = gfx::text(\"hi\", ivec2(0, 0), Color::White);
         print(w);
         print(gfx::text_width(\"a\\nbcd\"));
         print(gfx::text_width(\"\"));
         luna::draw(|| {});",
    );
    assert_eq!(machine.take_output(), ["8", "12", "0"]);
    assert!(machine.screen().pixels[..4 * 6].contains(&12));
}

#[test]
fn mouse_and_buttons() {
    let mut machine = machine(
        "luna::update(|| {
             if let pos? = input::mouse() {
                 print(f\"mouse {pos.x} {pos.y}\");
             } else {
                 print(\"no mouse\");
             }
             let left = input::held(Button::Left);
             let down = input::mouse_held(Mouse::Left);
             let click = input::mouse_pressed(Mouse::Right);
             let moved = input::mouse_moved();
             print(f\"{left} {down} {click} {moved}\");
         });
         luna::draw(|| {});",
    );
    let outside = Input {
        mouse: Some((WIDTH as i32, 10)),
        ..Input::default()
    };
    let corner = Input {
        mouse: Some((WIDTH as i32 - 1, HEIGHT as i32 - 1)),
        held: Button::Left.bit(),
        mouse_held: Mouse::Left.bit() | Mouse::Right.bit(),
        ..Input::default()
    };
    let inside = Input {
        mouse: Some((12, 34)),
        ..corner
    };
    let below = Input {
        mouse: Some((0, -1)),
        ..Input::default()
    };
    for input in [Input::default(), outside, below, corner, inside, inside] {
        machine.frame(&input);
    }
    assert_eq!(
        machine.take_output(),
        [
            "no mouse",
            "false false false false",
            "no mouse",
            "false false false true",
            "no mouse",
            "false false false true",
            "mouse 255 143",
            "true true true true",
            "mouse 12 34",
            "true true false true",
            "mouse 12 34",
            "true true false false",
        ]
    );
}

#[test]
fn taps_between_frames_are_delivered_once() {
    let mut machine = machine(
        "luna::update(|| {
             print((input::held(Button::A), input::pressed(Button::A), input::released(Button::A)));
             print((input::mouse_held(Mouse::Left), input::mouse_pressed(Mouse::Left)));
         });
         luna::draw(|| {});",
    );
    machine.frame(&Input {
        pressed: Button::A.bit(),
        released: Button::A.bit(),
        mouse_pressed: Mouse::Left.bit(),
        ..Input::default()
    });
    machine.frame(&Input::default());
    assert_eq!(
        machine.take_output(),
        [
            "[false, true, true]",
            "[false, true]",
            "[false, false, false]",
            "[false, false]"
        ]
    );
}

#[test]
fn button_names() {
    let bits = Button::ALL
        .iter()
        .fold(0, |bits, button| bits | button.bit());
    assert_eq!(bits, 0b1_1111_1111);
    assert_eq!(Mouse::Left.bit() | Mouse::Right.bit(), 0b11);
}

#[test]
fn random_is_seeded() {
    let src = "let i = 0;
               while i < 20 {
                   print(f\"{int::random(1000)} {float::random(2.0)}\");
                   i += 1;
               }
               luna::draw(|| {});";
    let mut a = machine(src);
    let mut b = machine(src);
    let lines = a.take_output();
    assert_eq!(lines.len(), 20);
    assert_eq!(lines, b.take_output());
    let mut other = Machine::load(&runner::cart(src), 2).unwrap();
    assert_ne!(lines, other.take_output());
}

#[test]
fn std_is_sandboxed() {
    let lines = top("let v = std::math::vec2(3.0, 4.0);
         print(v.length());
         print([3, 1, 2].reversed());
         print(\"a,b\".split(\",\").len());");
    assert_eq!(lines, ["5", "[2, 1, 3]", "2"]);
    for host in ["std::fs::cwd();", "use std::process;", "std::sys::arg(0);"] {
        let src = format!(
            "{host}
             luna::draw(|| {{}});"
        );
        assert_eq!(problems(&src).len(), 1, "{host}");
    }
}

#[test]
fn output_cap() {
    let lines = top("let i = 0;
         while i < 1500 {
             print(i);
             i += 1;
         }");
    assert_eq!(lines.len(), 1000);
    assert_eq!(lines[0], "500");
    assert_eq!(lines[999], "1499");
}
