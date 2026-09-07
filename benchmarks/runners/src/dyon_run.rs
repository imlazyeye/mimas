// mirrors dyon's own `dyonrun` example: dyon::run loads the script as a module with the
// standard library installed and invokes `main`; dyon::error reports any failure.
fn main() {
    let path = std::env::args().nth(1).expect("usage: dyon-run <script.dyon>");
    if dyon::error(dyon::run(&path)) {
        std::process::exit(1);
    }
}
