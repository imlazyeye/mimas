use std::fs;

fn main() {
    let path = std::env::args().nth(1).expect("usage: koto-run <script.koto>");
    let source = fs::read_to_string(&path).expect("failed to read script");

    let mut koto = koto::Koto::default();
    if let Err(e) = koto.compile_and_run(&source) {
        eprintln!("koto error: {e}");
        std::process::exit(1);
    }
}
