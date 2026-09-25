//! Runs the scripts that write the standard library's pages in the book. The scripts do the work;
//! this gives them the api to work from.

use std::path::Path;

use mimas::{
    Ty,
    library::Value,
    native,
    vm::{Vm, api::Api, conversion::Raisable},
};

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts");
    let mut files = vec![];
    for entry in std::fs::read_dir(&dir).expect("couldn't read the scripts directory") {
        let path = entry.expect("couldn't read a script").path();
        if path.extension().is_some_and(|ext| ext == "mim") {
            let name = path.to_string_lossy().into_owned();
            let source = std::fs::read_to_string(&path).expect("couldn't read a script");
            files.push((name, source));
        }
    }
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect();

    let result = Vm::compile_files(&files, install).and_then(|mut vm| Ok(vm.run()?));
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
