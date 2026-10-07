use std::{
    cell::{Ref, RefCell},
    collections::VecDeque,
    rc::Rc,
};

use mimas::{
    library::{Output, Random},
    vm::{FixtureRef, Vm},
};

use crate::{
    Cart, CartBuild, Diagnostic, DiagnosticKind, FUEL, Screen, WIDTH,
    api::{gfx::Color, input::Controls, install::install, luna::Luna},
    input::Input,
};

/// Primary driver for a [Cart].
pub struct Machine {
    vm: Vm,
    luna: FixtureRef<Luna>,
    screen: FixtureRef<RefCell<Screen>>,
    controls: FixtureRef<Controls>,
    cart: Cart,
    lines: Rc<RefCell<VecDeque<String>>>,
    previous: Input,
    fault: Option<Diagnostic>,
}

/// How many printed lines the machine keeps for the front end. The oldest go first.
const MAX_LINES: usize = 1000;

impl Machine {
    /// Compiles and starts a [Cart]. Takes a random seed to start it with.
    pub fn load(cart: &Cart, seed: u64) -> Result<Self, Vec<Diagnostic>> {
        let CartBuild {
            sheet,
            sources,
            script,
        } = cart.validate()?;
        let mut vm = Vm::compile_files(&sources, install).map_err(|error| {
            let kind = DiagnosticKind::Error;
            vec![Diagnostic::from_report(&error.0, cart, kind)]
        })?;

        let lines = Rc::new(RefCell::new(VecDeque::new()));
        let sink = Rc::clone(&lines);
        vm.fixture::<Output>().set(move |line| {
            let mut lines = sink.borrow_mut();
            if lines.len() == MAX_LINES {
                lines.pop_front();
            }
            lines.push_back(line.to_owned());
        });

        vm.fixture::<Random>().seed(seed);
        let luna = vm.fixture::<Luna>();
        let screen = vm.fixture::<RefCell<Screen>>();
        screen.borrow_mut().sheet = sheet;

        vm.set_fuel(Some(FUEL));
        if let Err(error) = vm.run() {
            return Err(vec![Diagnostic::from_report(
                &error,
                cart,
                DiagnosticKind::Fault,
            )]);
        }
        if luna.draw.borrow().is_none() {
            let message = "carts must provide a draw callback (`luna::draw`)";
            return Err(vec![Diagnostic::at(
                script,
                &cart.files[script],
                0..0,
                message,
            )]);
        }

        Ok(Self {
            controls: vm.fixture::<Controls>(),
            vm,
            luna,
            screen,
            cart: cart.clone(),
            lines,
            previous: Input::default(),
            fault: None,
        })
    }

    /// Executes a single frame of the cart. Runs the update hook (if there is one) followed by the
    /// draw hook. Returns if the Vm halted.
    pub fn frame(&mut self, input: &Input) -> bool {
        let (luna, screen) = (&*self.luna, &*self.screen);
        self.controls.feed(&self.previous, input);
        self.previous = *input;
        if self.fault.is_some() {
            return true;
        }

        screen.borrow_mut().reset_state();
        self.vm.set_fuel(Some(FUEL));
        luna.in_hook.set(true);
        let mut ran = Ok(());
        for slot in [&luna.update, &luna.draw] {
            let Some(hook) = slot.take() else { continue };
            ran = self.vm.call_value::<()>(&hook, ());
            *slot.borrow_mut() = Some(hook);
            if ran.is_err() {
                break;
            }
        }
        luna.in_hook.set(false);
        self.controls.clear_edges();

        match ran {
            Ok(()) => luna.tick.set(luna.tick.get() + 1),
            Err(error) => {
                // If we get an error we'll clear the screen and render the error
                let fault = Diagnostic::from_report(&error, &self.cart, DiagnosticKind::Fault);
                let mut screen = screen.borrow_mut();
                screen.reset_state();
                screen.clear(Color::Navy as u8);
                screen.text("halted", 4, 4, Color::Orange as u8);
                let chars: Vec<char> = fault.message.chars().collect();
                let mut y = 16;
                for line in chars.chunks((WIDTH - 8) / 4) {
                    screen.text(&line.iter().collect::<String>(), 4, y, Color::White as u8);
                    y += 6;
                }
                let place = format!("{}:{}:{}", fault.file, fault.line, fault.col);
                screen.text(&place, 4, y + 6, Color::Silver as u8);
                self.fault = Some(fault);
            }
        }
        false
    }

    /// The screen as the last frame left it.
    pub fn screen(&self) -> Ref<'_, Screen> {
        self.screen.borrow()
    }

    /// The fault that halted the machine, if one did.
    pub fn fault(&self) -> Option<&Diagnostic> {
        self.fault.as_ref()
    }

    /// How many frames have run.
    pub fn tick(&self) -> u64 {
        self.luna.tick.get()
    }

    /// Takes the lines the cart has printed since the last call, oldest first, without their
    /// newlines..
    pub fn take_output(&mut self) -> Vec<String> {
        self.lines.borrow_mut().drain(..).collect()
    }
}
