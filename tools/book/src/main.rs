//! Runs the book's preprocessors. Each is a script in `scripts`, compiled with the modules beside
//! it and the api the standard library's reference is generated from.

use std::path::Path;

use clap::Parser;
use mimas::{
    Ty,
    library::{ScriptArgs, Value},
    native,
    vm::{Vm, api::Api, conversion::Raisable},
};

#[derive(Parser)]
enum Book {
    /// Highlights the book's mimas code blocks.
    Highlight {
        /// Handed to the script as its own arguments.
        args: Vec<String>,
    },
    /// Adds the standard library's pages under `std.md`.
    Std,
}

fn main() {
    let (script, args) = match Book::parse() {
        Book::Highlight { args } => ("highlight.mim", args),
        Book::Std => ("std.mim", vec![]),
    };

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts");
    let script = dir.join(script);
    let mut files = vec![];
    for entry in std::fs::read_dir(&dir).expect("couldn't read the book's scripts") {
        let path = entry.expect("couldn't read a script").path();
        if path.extension().is_none_or(|ext| ext != "mim") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("couldn't read a script");
        if path == script || parse::lex::is_module(&source) {
            files.push((path.to_string_lossy().into_owned(), source));
        }
    }
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect();

    let result = Vm::compile_files(&files, install).and_then(|mut vm| {
        let mut script_args = vec![script.to_string_lossy().into_owned()];
        script_args.extend(args);
        vm.fixture::<ScriptArgs>().set(script_args);
        Ok(vm.run()?)
    });
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn install(api: &mut Api) {
    mimas::library::std(api);
    let mut docs = api.module("docs");
    docs.add(std_api);
    docs.add(display_ty);
}

/// The standard library's api as json.
#[native]
fn std_api() -> String {
    let library = Vm::new().install_library(mimas::library::std);
    serde_json::to_string(&library).expect("couldn't serialize the standard library")
}

/// Reads a type from its parsed json and writes it as mimas source would.
#[native]
fn display_ty(value: Value) -> Raisable<String> {
    serde_json::from_value::<Ty>(value.into())
        .map(|ty| ty.to_string())
        .into()
}
