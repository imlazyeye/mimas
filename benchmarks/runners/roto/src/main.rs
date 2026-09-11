use std::process::ExitCode;

use roto::Runtime;

fn main() -> ExitCode {
    let path = std::env::args().nth(1).expect("usage: roto-run <script.roto>");

    let mut runtime = Runtime::new();
    runtime.add_io_functions();

    let mut pkg = match runtime.compile(path.as_str()) {
        Ok(pkg) => pkg,
        Err(e) => {
            eprint!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let main = match pkg.get_function::<fn() -> ()>("main") {
        Ok(main) => main,
        Err(e) => {
            eprintln!("roto error: {e}");
            return ExitCode::FAILURE;
        }
    };
    main.call();

    ExitCode::SUCCESS
}
