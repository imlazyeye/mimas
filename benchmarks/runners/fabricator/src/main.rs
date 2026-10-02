// Lifted from fabricator's own `interpreter run` bin (crates/cli/src/bin/interpreter.rs): compile a
// GML chunk with the full stdlib installed, then run it on a fresh thread.
use std::{env, fs, path::Path};

use fabricator_compiler::{
    CompileSettings,
    compiler::{Compiler, ImportItems},
};
use fabricator_stdlib::StdlibContext as _;
use fabricator_vm as vm;

fn main() {
    let path = env::args().nth(1).expect("usage: gml-run <script.gml>");
    let code = fs::read_to_string(&path).expect("failed to read script");

    let interpreter = vm::Interpreter::new();
    interpreter.enter(|ctx| {
        let settings = CompileSettings::from_path(Path::new(&path)).set_optimization_passes(2);
        let output = Compiler::compile_chunk(
            ctx,
            "",
            ImportItems::with_magic(&ctx, ctx.stdlib()),
            settings,
            vm::SharedStr::new(&path),
            &code,
        )
        .expect("compile failed");
        let closure = vm::Closure::new(&ctx, output.chunk_prototype, None).unwrap();
        let thread = vm::Thread::new(&ctx);
        thread.run(ctx, closure).expect("runtime error");
    });
}
