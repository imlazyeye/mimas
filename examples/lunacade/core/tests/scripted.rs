#![allow(dead_code)]

use std::ops::RangeInclusive;

use lunacade_core::{Button, Input};
use rand::{RngExt, SeedableRng, rngs::StdRng};

pub struct Hold {
    pub button: Button,
    pub frames: RangeInclusive<u64>,
}

/// The input for the given tick in a run that plays itself. Nothing is held for the first 120
/// frames, then Start for 5, then every button is held and released on its own schedule, each
/// stretch lasting 8 to 90 frames. `holds` are held on top of that. The same `seed` gives the same
/// run.
pub fn scripted(seed: u64, frame: u64, holds: &[Hold]) -> Input {
    fn random_hold(seed: u64, button: Button, frame: u64) -> bool {
        // the nth stretch of a button's schedule, idle when n is even and held when it's odd
        fn stretch(seed: u64, button: Button, n: u64) -> u64 {
            let key = seed.rotate_left(32) ^ ((button as u64) << 24) ^ n;
            StdRng::seed_from_u64(key).random_range(8..=90)
        }

        let (mut end, mut n) = (0, 0);
        loop {
            end += stretch(seed, button, n);
            if frame < end {
                return n % 2 == 1;
            }
            n += 1;
        }
    }

    let mut input = Input::default();
    if (120..125).contains(&frame) {
        input.held |= Button::Start.bit();
    }
    if frame >= 125 {
        for button in Button::ALL {
            if random_hold(seed, button, frame - 125) {
                input.held |= button.bit();
            }
        }
    }
    for hold in holds.iter().filter(|hold| hold.frames.contains(&frame)) {
        input.held |= hold.button.bit();
    }
    input
}
