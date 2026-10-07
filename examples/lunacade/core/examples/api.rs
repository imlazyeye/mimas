//! Writes the cart API as a manifest, which `mimas docs` turns into the book's reference pages.
//!
//! ```text
//! api <out>
//! ```
use std::{path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let Some(out) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: api <out>");
        return ExitCode::FAILURE;
    };
    let written = match out.parent() {
        Some(dir) => std::fs::create_dir_all(dir),
        None => Ok(()),
    }
    .and_then(|()| mimas::write_api(&lunacade_core::library(), &out));
    match written {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}: {error}", out.display());
            ExitCode::FAILURE
        }
    }
}
