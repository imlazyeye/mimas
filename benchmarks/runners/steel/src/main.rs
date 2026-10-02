use std::fs;

use steel::steel_vm::engine::Engine;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: steel-run <script.scm>");
    let source = fs::read_to_string(&path).expect("failed to read script");

    let mut vm = Engine::new();
    if let Err(e) = vm.run(source) {
        eprintln!("steel error: {e}");
        std::process::exit(1);
    }
}
