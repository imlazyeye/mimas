use std::fs;

fn main() {
    let path = std::env::args().nth(1).expect("usage: mimas-run <script.mim>");
    let source = fs::read_to_string(&path).expect("failed to read script");
    if let Err(e) = mimas::Vm::execute(&source, mimas::library::std) {
        eprintln!("mimas error: {e}");
        std::process::exit(1);
    }
}
