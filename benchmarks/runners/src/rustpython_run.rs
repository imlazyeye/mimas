use std::{fs, process::exit};

use rustpython_vm::{compiler::Mode, Interpreter, Settings};

fn main() {
    let path = std::env::args().nth(1).expect("usage: rustpython-run <script.py>");
    let source = fs::read_to_string(&path).expect("failed to read script");

    let builder = Interpreter::builder(Settings::default());
    let defs = rustpython_stdlib::stdlib_module_defs(&builder.ctx);
    let interp = builder.add_native_modules(&defs).build();

    let exit_code = interp.run(|vm| {
        let scope = vm.new_scope_with_builtins();
        let code_obj = vm
            .compile(&source, Mode::Exec, path.clone())
            .map_err(|err| vm.new_syntax_error(&err, Some(&source)))?;
        vm.run_code_obj(code_obj, scope)?;
        Ok(())
    });

    exit(exit_code as i32);
}
