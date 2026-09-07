use std::sync::Arc;

use rune::runtime::VmResult;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Context, Diagnostics, Hash, Options, Source, Sources, Vm};

fn main() {
    let path = std::env::args().nth(1).expect("usage: rune-run <script.rn>");

    let context = Context::with_default_modules().expect("failed to build context");
    let runtime = Arc::new(context.runtime().expect("failed to build runtime"));

    let mut sources = Sources::new();
    sources
        .insert(Source::from_path(&path).expect("failed to read script"))
        .expect("failed to insert source");

    // match `rune run`: compile as a script (top-level statements are the entry) and execute that
    // top-level entry via Hash::EMPTY, rather than treating the file as a library of items.
    let mut options = Options::from_default_env().expect("failed to read options");
    options.script(true);

    let mut diagnostics = Diagnostics::new();
    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_options(&options)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Auto);
        diagnostics.emit(&mut writer, &sources).ok();
    }

    let unit = match result {
        Ok(unit) => unit,
        Err(_) => std::process::exit(1),
    };

    let mut vm = Vm::new(runtime, Arc::new(unit));
    let mut execution = match vm.execute(Hash::EMPTY, ()) {
        Ok(execution) => execution,
        Err(e) => {
            eprintln!("rune error: {e}");
            std::process::exit(1);
        }
    };
    if let VmResult::Err(e) = execution.complete() {
        eprintln!("rune error: {e}");
        std::process::exit(1);
    }
}
