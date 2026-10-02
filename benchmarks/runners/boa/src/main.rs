use std::path::Path;

use boa_engine::{Context, Source};
use boa_runtime::{console::DefaultLogger, Console};

fn main() {
    let path = std::env::args().nth(1).expect("usage: boa-run <script.js>");
    let mut ctx = Context::default();
    Console::register_with_logger(DefaultLogger, &mut ctx).expect("failed to register console");
    let source = Source::from_filepath(Path::new(&path)).expect("failed to read script");
    if let Err(e) = ctx.eval(source) {
        eprintln!("boa error: {e}");
        std::process::exit(1);
    }
}
