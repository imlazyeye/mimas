//! lunacade in a window of its own. It plays the cart built into it (see `build.rs`), or the cart
//! folder it's given, the way the page does: 60 frames a second, the screen scaled up by a whole
//! number, the same keys.
use std::{
    collections::HashMap,
    env,
    num::NonZeroU32,
    path::Path,
    process::ExitCode,
    rc::Rc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use lunacade_core::{Button, Cart, HEIGHT, Input, Machine, Mouse, PALETTE, WIDTH};
use softbuffer::{Context, Surface};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

include!(concat!(env!("OUT_DIR"), "/cart.rs"));

const FRAME: Duration = Duration::from_nanos(1_000_000_000 / 60);

/// How many cart frames run before the window is shown again. Time past that is dropped, so a slow
/// machine slows the game down instead of falling behind for good.
const MAX_FRAMES: usize = 2;

struct Console {
    title: String,
    machine: Machine,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    /// The keys that are down, each with the bit of the button it presses, since two keys can hold
    /// one button.
    keys: HashMap<KeyCode, u16>,
    input: Input,
    cursor: Option<PhysicalPosition<f64>>,
    last: Instant,
    owed: Duration,
}

/// How many window pixels a screen pixel takes, and where the screen's corner is in the window.
/// A window smaller than the screen shows nothing.
fn fit(size: PhysicalSize<u32>) -> Option<(usize, usize, usize)> {
    let (width, height) = (size.width as usize, size.height as usize);
    let scale = (width / WIDTH).min(height / HEIGHT);
    (scale > 0).then(|| {
        (
            scale,
            (width - WIDTH * scale) / 2,
            (height - HEIGHT * scale) / 2,
        )
    })
}

impl ApplicationHandler for Console {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attributes = Window::default_attributes()
            .with_title(&self.title)
            .with_inner_size(LogicalSize::new(WIDTH as u32 * 4, HEIGHT as u32 * 4))
            .with_min_inner_size(PhysicalSize::new(WIDTH as u32, HEIGHT as u32));
        let window = Rc::new(event_loop.create_window(attributes).unwrap());
        let context = Context::new(Rc::clone(&window)).unwrap();
        self.surface = Some(Surface::new(&context, window).unwrap());
        self.last = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let input = &mut self.input;
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                let button = match code {
                    KeyCode::ArrowUp | KeyCode::KeyW => Button::Up,
                    KeyCode::ArrowDown | KeyCode::KeyS => Button::Down,
                    KeyCode::ArrowLeft | KeyCode::KeyA => Button::Left,
                    KeyCode::ArrowRight | KeyCode::KeyD => Button::Right,
                    KeyCode::KeyZ | KeyCode::KeyJ => Button::A,
                    KeyCode::KeyX | KeyCode::KeyK => Button::B,
                    KeyCode::KeyC | KeyCode::KeyL => Button::X,
                    KeyCode::KeyV | KeyCode::Semicolon => Button::Y,
                    KeyCode::Enter => Button::Start,
                    _ => return,
                };
                if event.state.is_pressed() {
                    self.keys.insert(code, button.bit());
                } else {
                    self.keys.remove(&code);
                }
                let before = input.held;
                input.held = self.keys.values().fold(0, |held, bit| held | bit);
                input.pressed |= input.held & !before;
                input.released |= before & !input.held;
            }
            WindowEvent::Focused(false) => {
                self.keys.clear();
                input.released |= input.held;
                input.held = 0;
                input.mouse_held = 0;
            }
            WindowEvent::CursorMoved { position, .. } => self.cursor = Some(position),
            WindowEvent::CursorLeft { .. } => self.cursor = None,
            WindowEvent::MouseInput { state, button, .. } => {
                let bit = match button {
                    MouseButton::Left => Mouse::Left.bit(),
                    MouseButton::Right => Mouse::Right.bit(),
                    _ => return,
                };
                if state == ElementState::Pressed {
                    input.mouse_pressed |= bit & !input.mouse_held;
                    input.mouse_held |= bit;
                } else {
                    input.mouse_held &= !bit;
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(surface) = &mut self.surface else {
                    return;
                };
                let size = surface.window().inner_size();
                let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                else {
                    return;
                };
                surface.resize(width, height).unwrap();
                let mut buffer = surface.buffer_mut().unwrap();
                buffer.fill(0);
                if let Some((scale, left, top)) = fit(size) {
                    let screen = self.machine.screen();
                    let mut line = vec![0; WIDTH * scale];
                    for (y, row) in screen.pixels.chunks_exact(WIDTH).enumerate() {
                        for (index, pixels) in row.iter().zip(line.chunks_exact_mut(scale)) {
                            let [r, g, b] = PALETTE[(index & 15) as usize];
                            pixels.fill(u32::from_be_bytes([0, r, g, b]));
                        }
                        for y in top + y * scale..top + (y + 1) * scale {
                            let start = y * size.width as usize + left;
                            buffer[start..start + line.len()].copy_from_slice(&line);
                        }
                    }
                }
                buffer.present().unwrap();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(surface) = &self.surface else {
            return;
        };
        let window = surface.window();
        let now = Instant::now();
        self.owed += (now - self.last).min(Duration::from_millis(100));
        self.last = now;

        let placed = fit(window.inner_size()).zip(self.cursor);
        self.input.mouse = placed.and_then(|((scale, left, top), cursor)| {
            let x = ((cursor.x - left as f64) / scale as f64).floor() as i32;
            let y = ((cursor.y - top as f64) / scale as f64).floor() as i32;
            let on_screen = (0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y);
            on_screen.then_some((x, y))
        });

        for _ in 0..MAX_FRAMES {
            if self.owed < FRAME {
                break;
            }
            self.owed -= FRAME;
            let halted = self.machine.frame(&self.input);
            self.input.pressed = 0;
            self.input.released = 0;
            self.input.mouse_pressed = 0;
            if halted {
                continue;
            }
            for line in self.machine.take_output() {
                println!("{line}");
            }
            if let Some(fault) = self.machine.fault() {
                eprintln!(
                    "{}:{}:{} {}",
                    fault.file, fault.line, fault.col, fault.message
                );
            }
            window.request_redraw();
        }
        self.owed = self.owed.min(FRAME);
        event_loop.set_control_flow(ControlFlow::WaitUntil(now + FRAME - self.owed));
    }
}

fn main() -> ExitCode {
    let (title, cart) = match (EMBEDDED, env::args().nth(1)) {
        (Some((name, files)), _) => {
            let files = files
                .iter()
                .map(|(path, text)| (path.to_string(), text.to_string()));
            (
                name.to_owned(),
                Cart {
                    files: files.collect(),
                },
            )
        }
        (None, Some(dir)) => match Cart::from_dir(Path::new(&dir)) {
            Ok(cart) => {
                let name = Path::new(&dir).file_name().and_then(|name| name.to_str());
                (name.unwrap_or("lunacade").to_owned(), cart)
            }
            Err(error) => {
                eprintln!("{dir}: {error}");
                return ExitCode::FAILURE;
            }
        },
        (None, None) => {
            eprintln!("usage: lunacade-native <cart dir>");
            return ExitCode::FAILURE;
        }
    };

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos());
    let mut machine = match Machine::load(&cart, seed as u64) {
        Ok(machine) => machine,
        Err(problems) => {
            for problem in problems {
                let place = format!("{}:{}:{}", problem.file, problem.line, problem.col);
                eprintln!("{place} {}", problem.message);
            }
            return ExitCode::FAILURE;
        }
    };
    for line in machine.take_output() {
        println!("{line}");
    }

    let mut console = Console {
        title,
        machine,
        surface: None,
        keys: HashMap::new(),
        input: Input::default(),
        cursor: None,
        last: Instant::now(),
        owed: Duration::ZERO,
    };
    match EventLoop::new().and_then(|event_loop| event_loop.run_app(&mut console)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
