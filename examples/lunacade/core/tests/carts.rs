mod scripted;

use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use lunacade_core::{Button, Cart, Diagnostic, Machine};
use scripted::{Hold, scripted};

const FRAMES: u64 = 3600;

const STRESS_FRAMES: u64 = 600;

fn only() -> Vec<String> {
    let names = std::env::var("LUNACADE_CARTS").unwrap_or_default();
    let names = names.split(',').map(str::trim);
    names
        .filter(|name| !name.is_empty())
        .map(String::from)
        .collect()
}

fn carts() -> Vec<(String, PathBuf)> {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../carts"));
    let paths: Vec<PathBuf> = match fs::read_dir(root) {
        Ok(entries) => entries.map(|entry| entry.unwrap().path()).collect(),
        Err(error) if error.kind() == ErrorKind::NotFound => Vec::new(),
        Err(error) => panic!("{}: {error}", root.display()),
    };
    let mut carts: Vec<(String, PathBuf)> = paths
        .into_iter()
        .filter(|path| path.is_dir())
        .map(|path| {
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                path,
            )
        })
        .collect();
    carts.sort();

    let only = only();
    if !only.is_empty() {
        carts.retain(|(name, _)| only.contains(name));
    }
    carts
}

/// Plays the cart with the scripted input and gives back what it printed, or why it stopped.
fn play(dir: &Path, holds: &[Hold], frames: u64) -> Result<Vec<String>, String> {
    let cart = Cart::from_dir(dir).map_err(|error| error.to_string())?;
    let mut machine = Machine::load(&cart, 1).map_err(|problems| {
        let lines: Vec<String> = problems.iter().map(describe).collect();
        lines.join("\n")
    })?;
    let mut lines = Vec::new();
    for frame in 0..frames {
        machine.frame(&scripted(1, frame, holds));
        if let Some(fault) = machine.fault() {
            return Err(format!("halted on frame {frame} at {}", describe(fault)));
        }
        lines.extend(machine.take_output());
    }
    Ok(lines)
}

fn describe(diagnostic: &Diagnostic) -> String {
    format!(
        "{}:{}:{}: {}",
        diagnostic.file, diagnostic.line, diagnostic.col, diagnostic.message
    )
}

#[test]
fn carts_run() {
    let carts = carts();
    for name in only() {
        let found = carts.iter().any(|(cart, _)| *cart == name);
        assert!(found, "LUNACADE_CARTS names `{name}`, which isn't a cart");
    }
    let mut failures = Vec::new();
    for (name, dir) in carts {
        match play(&dir, &[], FRAMES) {
            Ok(lines) => println!("{name}: ran {FRAMES} frames, printed {} lines", lines.len()),
            Err(message) => {
                println!("{name}: failed");
                failures.push(format!("{name}: {message}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn swarm_stress() {
    let Some((_, dir)) = carts().into_iter().find(|(name, _)| name == "swarm") else {
        println!("no swarm cart, skipping its stress run");
        return;
    };
    let holds = [Button::X, Button::Y].map(|button| Hold {
        button,
        frames: 60..=90,
    });
    let lines =
        play(&dir, &holds, STRESS_FRAMES).unwrap_or_else(|message| panic!("swarm: {message}"));
    let crowd = lines
        .iter()
        .filter_map(|line| line.strip_prefix("enemies ")?.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    println!("swarm stress: {crowd} enemies at most");
    assert!(crowd >= 300, "the crowd peaked at {crowd} enemies");
}

#[test]
fn chess_thinks() {
    let Some((_, dir)) = carts().into_iter().find(|(name, _)| name == "chess") else {
        println!("no chess cart, skipping its opening move");
        return;
    };
    // Start, then A on the king's pawn, up twice and A again, all before the scripted input begins
    let presses = [
        (Button::Start, 5),
        (Button::A, 20),
        (Button::Up, 24),
        (Button::Up, 28),
        (Button::A, 32),
    ];
    let holds = presses.map(|(button, frame)| Hold {
        button,
        frames: frame..=frame + 1,
    });
    play(&dir, &holds, 120).unwrap_or_else(|message| panic!("chess: {message}"));
}
