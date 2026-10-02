fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: rhai-run <script.rhai>");
    let mut engine = rhai::Engine::new();
    engine.set_optimization_level(rhai::OptimizationLevel::Full);
    if let Err(e) = engine.run_file(path.into()) {
        eprintln!("rhai error: {e}");
        std::process::exit(1);
    }
}
