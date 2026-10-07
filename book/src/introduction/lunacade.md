# lunacade

lunacade is a fantasy console for mimas, a small imaginary machine whose games are mimas programs. These programs are called carts. A cart draws on a 256 by 144 screen with 16 colors and reads a d-pad, four buttons, Start and a mouse. Five example carts ship with it, from Pong to a survivors-like called Swarm.

You can play them, edit them and make your own in the browser at [mim.as/lunacade](../lunacade/). Like the rest of mimas it's young, and it has rough edges.

lunacade is in the book as an example of embedding the VM directly. [The Bevy plugin](../extension/bevy.md) lets Bevy run scripts for you, but lunacade's `Machine` owns a `Vm` itself and calls a cart's `update` and `draw` each frame. See [The host](#the-host) for how that fits together. Its code is in [`examples/lunacade`](https://github.com/imlazyeye/mimas/tree/main/examples/lunacade).

## On the web

The page has the console on one side, with a picker for the example carts under it, and two tabs on the other.

| Tab | What it does |
| :-- | :-- |
| Code | Edits a cart's files. It checks the cart as you type, marks the errors, and runs the cart again a moment after the code checks clean (<kbd>Ctrl</kbd>+<kbd>Enter</kbd> runs it now). |
| Sprites | A pixel editor for `sprites.txt`. |

The [Cart API](./lunacade-api.md) chapter is the reference for everything a cart can call. Edits are saved in your browser for each cart. Share compresses the whole cart into the part of the link after the `#`. Nothing is sent to a server, and the page sets no cookies. The page needs WebAssembly, and it loads its code editor and the font of its title from CDNs.

## A cart

A cart is a folder of files, or the same files on the page.

| File | What it is |
| :-- | :-- |
| One `.mim` file | The cart's script, under any name. Its top-level code registers `luna::update` and `luna::draw`. |
| Other `.mim` files | [Modules](../reference/modules.md) that the script can use. |
| `sprites.txt` | The sprite sheet, as text. It's optional. |

A cart has one script, which is the `.mim` file that doesn't open with `module`. That's how `mimas run` picks the script out of a folder, and a cart with a second script or with none is an error. Here's a whole script:

```mimas ignore
use std::math::{IVec2, ivec2};

const SHIP_SPRITE = 0;

struct Ship {
    pos: IVec2,
}

impl Ship {
    fn update(self) {
        if input::held(Button::Left) {
            self.pos.x -= 1;
        }
        if input::held(Button::Right) {
            self.pos.x += 1;
        }
    }

    fn draw(self) {
        gfx::clear(Color::Navy);
        gfx::sprite(SHIP_SPRITE, self.pos);
        gfx::text("left and right", ivec2(2, 2), Color::White);
    }
}

let ship = Ship { pos = ivec2(124, 68) };
luna::update(|| ship.update());
luna::draw(|| ship.draw());
```

The console calls the cart's `update` and then its `draw` 60 times a second. `luna::draw` is required and `luna::update` is optional. Both only register in top-level code, and registering one inside a hook is a fault. Nothing clears the screen between frames. A draw usually starts with `gfx::clear`.

The cart keeps its state in one struct because a top-level `fn` can't see a top-level `let`, and a [closure](../reference/functions.md#closures) copies each binding it captures when it's made. A struct is shared (both closures above use the same `ship`).

### Sprites

The sprite sheet is 128 by 128 pixels, which is 16 by 16 sprites of 8 by 8. Sprite `n` is in column `n % 16` and row `n ~/ 16`, and `gfx::sprite(n, pos)` draws it. `sprites.txt` is up to 128 lines, one for each row of the sheet, with a hex digit for each of up to 128 pixels. The digit is the pixel's place in the palette. Lines that are short or missing are filled with `0`, and any other character is an error that names its line and column. The ship above is the first sprite on the sheet:

```text
000cc000
00acca00
0acccca0
acccccca
ac0cc0ca
a003300a
00044000
```

Sprites skip color `0`, black, by default, which is what the empty corners above are. `gfx::transparent` picks another color, or none.

### The palette

Every cart draws with [Sweetie 16](https://lospec.com/palette-list/sweetie-16), which is fixed. In code the colors are the variants of `Color`, and `Color::to_int` and `Color::from_int` convert them to and from their digit.

| Digit | Color | Hex | Digit | Color | Hex |
| :-- | :-- | :-- | :-- | :-- | :-- |
| `0` | Black | `#1a1c2c` | `8` | Navy | `#29366f` |
| `1` | Plum | `#5d275d` | `9` | Blue | `#3b5dc9` |
| `2` | Red | `#b13e53` | `a` | Sky | `#41a6f6` |
| `3` | Orange | `#ef7d57` | `b` | Cyan | `#73eff7` |
| `4` | Yellow | `#ffcd75` | `c` | White | `#f4f4f4` |
| `5` | Lime | `#a7f070` | `d` | Silver | `#94b0c2` |
| `6` | Green | `#38b764` | `e` | Slate | `#566c86` |
| `7` | Teal | `#257179` | `f` | Charcoal | `#333c57` |

## The API

Carts get three modules and three types, and all of them are in scope without a `use`. The [Cart API](./lunacade-api.md) chapter documents each of them, and is generated from the console's own API when the book is built.

| Name | What it has |
| :-- | :-- |
| `luna` | `update` and `draw`, which register the hooks, `frame`, and the constants `WIDTH` and `HEIGHT`. |
| `gfx` | Drawing: `clear`, `pixel`, `line`, `rect`, `circle`, `sprite`, `blit` and `text`, plus `camera`, `clip`, `remap` and `transparent`. Coordinates are `int` pixels. |
| `input` | `held`, `pressed` and `released` for a `Button`, `mouse`, and `mouse_held` and `mouse_pressed` for a `Mouse`. |
| `Color`, `Button`, `Mouse` | The enums the modules take. |

Every drawing call clips to the screen. Drawing off it isn't an error. Each native's doc comment says how it treats values that are out of range.

Carts also get the [sandboxed std](../extension/advanced-usage.md#installer-functions): `math`, `parse`, and the methods on strings, arrays and dicts, but not `fs`, `process` or `sys`. The console seeds std's random numbers (`int::random`, `float::random`, `bool::random`, `array.shuffle` and `array.choose`) when it loads a cart, so the same seed and the same input play the same game.

## Controls

lunacade reads a keyboard and a mouse.

| Button | Keyboard |
| :-- | :-- |
| Up, Down, Left, Right | Arrow keys or <kbd>W</kbd> <kbd>A</kbd> <kbd>S</kbd> <kbd>D</kbd> |
| A | <kbd>Z</kbd> or <kbd>J</kbd> |
| B | <kbd>X</kbd> or <kbd>K</kbd> |
| X | <kbd>C</kbd> or <kbd>L</kbd> |
| Y | <kbd>V</kbd> or <kbd>;</kbd> |
| Start | <kbd>Enter</kbd> |

The mouse is the cursor over the screen, in screen pixels. `input::mouse()` is `null` while the cursor is outside it. Click the console first so it has the keyboard.

The console works out which buttons were just pressed or released by comparing each frame's state with the one before.

## The fuel cap

A cart's `update` and `draw` run to completion every frame, and a heavy frame makes the page slower rather than being cut short. The one limit is a cap of two million VM ops on a frame, and on the cart's top-level code. A frame that goes past it faults with "ran out of fuel", which is what happens to a `loop {}`. A fault halts the cart until it's loaded again, and the console shows the message and where it happened.

## The example carts

Pong, Snake, Breakout and Asteroids are each one script. Swarm is a survivors-like split into modules, and it's written to be read. Its six weapons implement a `Weapon` [pact](../reference/pacts.md), what a level-up offers is an `Offer` enum with payloads that gets matched exhaustively, and looking for the nearest enemy gives back an option. Hold X and Y together on its title screen for half a second to start a run with 300 enemies already on the field and a player who can't be hurt.

They're in `examples/lunacade/carts`, and the page has them all. A new cart on the page starts as the one in `carts/new`.

## The host

`lunacade-core` is the console without a page, and it has no dependency on the web. Its `Machine` loads a cart, and a front end calls `frame` 60 times a second with what the player is holding:

```rs
use lunacade_core::{Button, Cart, Frame, Input, Machine};

let cart = Cart::from_dir("carts/pong".as_ref()).unwrap();
let mut machine = Machine::load(&cart, seed).expect("the cart loads");

// then, 60 times a second
let input = Input { held: Button::Left.bit(), ..Input::default() };
if machine.frame(&input) == Frame::Ran {
    machine.screen().write_rgba(&mut pixels);
}
```

`load` checks the cart's files and then compiles them against the sandboxed std and the cart API, which a host installs the way [Installer Functions](../extension/advanced-usage.md#installer-functions) describe. Everything in the API is registered explicitly with `#[native]` and `Api`, never with `#[mimas]`. `load` also points `print` at a buffer the front end reads, runs the cart's top-level code, and gives back the problems if the cart never registers a `luna::draw`.

`luna::update` and `luna::draw` take a closure and keep it with `ctx.stash`, which is the host side of [Function Values](../extension/function-values.md), and `frame` calls them back with `vm.call_value`. The cap on a frame is `Vm::set_fuel`, which makes a script that runs too many ops fault with "ran out of fuel" and leaves the Vm usable afterwards.

`frame` never reads a clock, and the front end times its own frames. On the page that front end is a few hundred lines of JavaScript: the `lunacade` crate wraps a `Machine` in a `Console` class through wasm-bindgen, and the page draws the screen it copies out into a canvas, reads the keyboard and mouse, and runs the frames from `requestAnimationFrame`. The crate also checks a cart against the installed API for the editor, and ships the example carts. `cargo test -p lunacade-core` plays every cart in `carts` for a minute of frames with scripted input.

```admonish note title="Why not the Bevy plugin?"
The plugin is for scripts that read and write a game's components and resources, with Bevy's schedule running them. A cart has one screen and a cap on its ops, and none of that lives in a Bevy world. lunacade also needs the fuel limit and its own say over when each hook runs, which the plugin doesn't offer.
```

## Credits

The palette is Sweetie 16 by GrafxKid, at [lospec.com/palette-list/sweetie-16](https://lospec.com/palette-list/sweetie-16). The font and the carts were made for lunacade.
