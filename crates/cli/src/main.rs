use api::Library;
use clap::Parser;
use colored::Colorize;
use num_format::{Locale, ToFormattedString};
use parse::Ast;
use solve::Modules;
use std::{path::PathBuf, time::Duration};
use vm::conversion::Raisable;

mod ice;
mod input;
mod render;
mod unit;
pub use input::*;
use unit::Unit;

const ICE_EXIT_CODE: i32 = 101;
type Scripts<'a> = Vec<&'a (PathBuf, String)>;
const DOCS_SCRIPT: &str = include_str!("../scripts/docs.mim");
const MDBOOK_SCRIPT: &str = include_str!("../scripts/mdbook.mim");

fn main() {
    ice::install_hook();
    let input = Cli::parse_from(massage_args(std::env::args().collect()));
    if input.color {
        unsafe {
            std::env::set_var("CLICOLOR_FORCE", "1");
        }
    }
    let status_code = match input.command {
        Some(Commands::Check { path }) => check(path, input.color),
        Some(Commands::Build { path }) => build(path, input.color, input.dump_bytes, input.dump_ir),
        Some(Commands::Run { path, script_args }) => run(
            path,
            script_args,
            input.color,
            input.dump_bytes,
            input.dump_ir,
            input.time,
        ),
        Some(Commands::Repl { path }) => repl(path, input.color),
        Some(Commands::Docs {
            output_path,
            renderer: _,
            manifest,
            include_std: std,
            mdbook,
        }) => docs(output_path, manifest, std, mdbook, input.color),
        None => repl(None, input.color),
    };
    std::process::exit(status_code);
}

fn check(path: Option<PathBuf>, color: bool) -> i32 {
    let timer = std::time::Instant::now();
    let unit = Unit::new(&resolve_path(path));
    let library = host_library(&unit);
    let (modules, scripts, mut count) = match load(&unit, &library, color) {
        Ok(loaded) => loaded,
        Err(code) => return code,
    };
    for file in scripts {
        match solve(&modules, file, color) {
            Ok(script) => count += script.errors.len(),
            Err(code) => return code,
        }
    }
    let total_duration = timer.elapsed();

    let seperator = "-".repeat(50);
    println!("{seperator}");
    println!(
        "  {}",
        format!(
            "Found {} error{}.",
            count.to_string().bright_red().bold(),
            if count == 1 { "" } else { "s" },
        )
        .bold()
    );
    println!(
        "  {}",
        format!(
            "Ran on {} lines in {}.",
            unit.lines().to_formatted_string(&Locale::en),
            format_duration(total_duration),
        )
        .italic()
        .bright_black()
    );
    println!("{seperator}");

    i32::from(count > 0)
}

fn build(path: Option<PathBuf>, color: bool, disasm: bool, dump_ir: bool) -> i32 {
    let timer = std::time::Instant::now();
    let unit = Unit::new(&resolve_path(path));
    let library = host_library(&unit);
    let (modules, files, mut count) = match load(&unit, &library, color) {
        Ok(loaded) => loaded,
        Err(code) => return code,
    };
    let mut scripts = vec![];
    for file in files {
        match solve(&modules, file, color) {
            Ok(script) => {
                count += script.errors.len();
                scripts.push(script);
            }
            Err(code) => return code,
        }
    }
    if count > 0 {
        return 1;
    }
    for script in scripts {
        if let Err(code) = compile(&modules, script, &library, disasm, dump_ir) {
            return code;
        }
    }

    let seperator = "-".repeat(50);
    println!("{seperator}");
    println!(
        "  {}",
        format!(
            "Compiled {} lines in {}.",
            unit.lines().to_formatted_string(&Locale::en),
            format_duration(timer.elapsed()),
        )
        .italic()
        .bright_black()
    );
    println!("{seperator}");
    0
}

fn run(
    path: Option<PathBuf>,
    script_args: Vec<String>,
    color: bool,
    disasm: bool,
    dump_ir: bool,
    time: bool,
) -> i32 {
    let timer = std::time::Instant::now();
    let path = resolve_path(path);
    let unit = Unit::new(&path);
    let mut vm = vm::Vm::new();
    let library = vm.install_library(library::std);
    let (modules, scripts) = match load(&unit, &library, color) {
        Ok((modules, scripts, 0)) => (modules, scripts),
        Ok(_) => return 1,
        Err(code) => return code,
    };
    let file = match scripts[..] {
        [file] => file,
        [] => {
            let error = "error".bright_red().bold();
            println!("{error}: {} has no script to run", path.display());
            return 1;
        }
        _ => {
            let names: Vec<_> = scripts
                .iter()
                .map(|(file, _)| file.display().to_string())
                .collect();
            let error = "error".bright_red().bold();
            println!(
                "{error}: {} has several scripts, pick one to run: {}",
                path.display(),
                names.join(", ")
            );
            return 1;
        }
    };
    let script = match solve(&modules, file, color) {
        Ok(script) if script.errors.is_empty() => script,
        Ok(_) => return 1,
        Err(code) => return code,
    };
    let compiled = match compile(&modules, script, &library, disasm, dump_ir) {
        Ok(compiled) => compiled,
        Err(code) => return code,
    };

    let mut resolved_args = Vec::with_capacity(script_args.len() + 1);
    resolved_args.push(file.0.to_string_lossy().into_owned());
    resolved_args.extend(script_args);
    vm.fixture::<library::ScriptArgs>().set(resolved_args);

    let result = ice::catch("execution", move || {
        vm.load_program(compiled);
        vm.run()
    });
    match result {
        Err(report) => {
            report.emit();
            ICE_EXIT_CODE
        }
        Ok(Err(report)) => {
            render::emit(report.as_ref(), color);
            1
        }
        Ok(Ok(())) => {
            if time {
                eprintln!(
                    "{}",
                    format!("ran in {}", format_duration(timer.elapsed()))
                        .italic()
                        .bright_black()
                );
            }
            0
        }
    }
}

fn repl(path: Option<PathBuf>, color: bool) -> i32 {
    use rustyline::error::ReadlineError;
    use std::sync::atomic::Ordering;

    let unit = Unit::new(&resolve_path(path));
    let mut vm = vm::Vm::new();
    let library = vm.install_library(library::std);
    let modules = match load(&unit, &library, color) {
        Ok((modules, _, 0)) => modules,
        Ok(_) => return 1,
        Err(code) => return code,
    };
    let mut session = vm::Session::new(vm, library, modules);
    let error = "error".bright_red().bold();
    let interrupt = session.interrupt();
    if let Err(e) = ctrlc::set_handler(move || interrupt.store(true, Ordering::Relaxed)) {
        eprintln!("{error}: can't listen for ctrl-c: {e}");
        return 1;
    }
    let mut editor = match rustyline::DefaultEditor::new() {
        Ok(editor) => editor,
        Err(e) => {
            eprintln!("{error}: {e}");
            return 1;
        }
    };
    let mut input = String::new();
    loop {
        let prompt = if input.is_empty() { "> " } else { ". " };
        match editor.readline(prompt) {
            Ok(line) => {
                input.push_str(&line);
                input.push('\n');
                if vm::Session::unfinished(&input) {
                    continue;
                }
                let text = std::mem::take(&mut input);
                if text.trim().is_empty() {
                    continue;
                }
                let _ = editor.add_history_entry(text.trim_end());
                match ice::catch("repl", || session.run(&text)) {
                    Err(report) => {
                        report.emit();
                        return ICE_EXIT_CODE;
                    }
                    Ok(Ok(Some(echo))) => println!("{echo}"),
                    Ok(Ok(None)) => {}
                    Ok(Err(report)) => render::emit(report.as_ref(), color),
                }
            }
            Err(ReadlineError::Interrupted) => input.clear(),
            Err(ReadlineError::Eof) => return 0,
            Err(e) => {
                eprintln!("{error}: {e}");
                return 1;
            }
        }
    }
}

fn docs(
    output_path: Option<PathBuf>,
    manifest: Option<PathBuf>,
    std: bool,
    mdbook: Option<String>,
    color: bool,
) -> i32 {
    // mdbook asks `supports <renderer>` before running us, and markdown pages suit every renderer
    if mdbook.is_some() && output_path.is_some() {
        return 0;
    }

    // compiled before the lookup, since this writes our own manifest (which is what a lookup in
    // crates/cli finds)
    let files = [("docs.mim", DOCS_SCRIPT), ("mdbook.mim", MDBOOK_SCRIPT)];
    let compiled = vm::Vm::compile_files(&files, |api| {
        library::std(api);
        api.module("docs").add(display_ty);
    });

    let error = "error".bright_red().bold();
    let manifest = match manifest {
        Some(path) => api::Manifest::read(&path).map(Some),
        None => api::Manifest::find(&api::Project::of(&resolve_path(None))),
    };
    let host = match manifest {
        Ok(Some(manifest)) => manifest.library,
        Ok(None) if std => std_alone(),
        // mdbook builds the book whether or not the host has been run, so the chapter is left
        // as it is rather than failing the build
        Err(problem) if mdbook.is_some() => {
            let warning = "warning".bright_yellow().bold();
            eprintln!("{warning}: {problem}, so the chapter is left as it is");
            let input: Vec<serde_json::Value> = match serde_json::from_reader(std::io::stdin()) {
                Ok(input) => input,
                Err(problem) => {
                    eprintln!("{error}: {problem}");
                    return 1;
                }
            };
            let Some(book) = input.into_iter().nth(1) else {
                eprintln!("{error}: mdbook sent no book");
                return 1;
            };
            println!("{book}");
            return 0;
        }
        Ok(None) => {
            eprintln!(
                "{error}: no manifest found. Run your host with `cargo run` to write one, or pass \
                 `--include-std` to document the standard library."
            );
            return 1;
        }
        Err(problem) => {
            eprintln!("{error}: {problem}. Run your host with `cargo run` to write it.");
            return 1;
        }
    };
    // every adt keeps its name, since the host's own types can name std's
    let names: Vec<_> = host
        .adts()
        .iter()
        .map(|adt| (adt.adt_id, adt.name.clone()))
        .collect();
    let host = if std { host } else { host.without_std() };
    let json = serde_json::to_value(&host).expect("couldn't serialize the manifest");

    let result = compiled.and_then(|mut vm| {
        // solving docs.mim named its own adts over the ids the manifest's adts use
        for (id, name) in &names {
            shared::name_adt(id.index(), name);
        }
        vm.run()?;
        let json = library::Value::from(json);
        match (mdbook, output_path) {
            (Some(chapter), _) => Ok(vm.call("preprocess", (json, chapter))?),
            (None, Some(folder)) => Ok(vm.call("markdown", (json, folder.display().to_string()))?),
            (None, None) => unreachable!("clap requires one"),
        }
    });
    match result {
        Ok(()) => 0,
        Err(e) => {
            render::emit(e.0.as_ref(), color);
            1
        }
    }
}

/// Reads a type from its parsed json and writes it as mimas source would.
#[vm::native]
fn display_ty(value: library::Value) -> Raisable<String> {
    serde_json::from_value::<shared::Ty>(value.into())
        .map(|ty| ty.to_string())
        .into()
}

/// Loads the unit's project: its modules solved together as one library, and the scripts the path
/// names (the file itself, or every one in the directory), each of which is solved on top of the
/// modules and sees nothing of the others. Prints the errors outside the scripts, counting them,
/// or says with which exit code that crashed.
fn load<'a>(
    unit: &'a Unit,
    library: &Library<()>,
    color: bool,
) -> Result<(Modules, Scripts<'a>, usize), i32> {
    report_meta_errors(&unit.io_errors);
    let (modules, scripts): (Vec<_>, Scripts) = unit
        .files
        .iter()
        .partition(|(_, text)| parse::lex::is_module(text));
    let modules = ice::catch("check", || {
        Modules::from_files(
            modules.iter().map(|(path, text)| (path, text.as_str())),
            library,
        )
    })
    .map_err(|report| {
        report.emit();
        ICE_EXIT_CODE
    })?;

    for error in &modules.errors {
        render::emit(error.as_ref(), color);
    }
    let count = unit.io_errors.len() + modules.errors.len();
    let scripts = scripts
        .into_iter()
        .filter(|(path, _)| unit.file.as_ref().is_none_or(|file| file == path))
        .collect();
    return Ok((modules, scripts, count));

    fn report_meta_errors(io_errors: &[std::io::Error]) {
        if !io_errors.is_empty() {
            println!(
                "\n{}: The following errors occurred while trying to read your project's files...",
                "error".bright_red().bold()
            );
            io_errors.iter().for_each(|error| {
                println!("{error}");
            })
        }
    }
}

/// Solves a script on top of the modules and prints its errors, or says with which exit code that
/// crashed.
fn solve(modules: &Modules, (path, text): &(PathBuf, String), color: bool) -> Result<Modules, i32> {
    let script =
        ice::catch("check", || modules.load([(path, text.as_str())])).map_err(|report| {
            report.emit();
            ICE_EXIT_CODE
        })?;
    for error in &script.errors {
        render::emit(error.as_ref(), color);
    }
    Ok(script)
}

/// Compiles a script on top of the modules, or says with which exit code that crashed.
fn compile(
    modules: &Modules,
    script: Modules,
    library: &Library<()>,
    disasm: bool,
    dump_ir: bool,
) -> Result<compile::Program, i32> {
    let Modules { asts, solver, .. } = script;
    let intrinsics = library.intrinsics().clone();
    ice::catch("compilation", || {
        let mut ir = compile::Ir::new(solve::Resolutions::from(solver), intrinsics);
        ir.lower(modules.asts.iter().chain(&asts).flat_map(Ast::stmts));
        if dump_ir {
            println!("{ir}");
        }
        compile::Compiler::new()
            .with_disasm(disasm)
            .compile(&mut ir)
    })
    .map_err(|report| {
        report.emit();
        ICE_EXIT_CODE
    })
}

// bare `mimas foo.mim` means `mimas run foo.mim`; inject `run` when the first
// positional isn't already a subcommand. `mimas` alone opens the repl.
fn massage_args(mut args: Vec<String>) -> Vec<String> {
    const SUBCOMMANDS: [&str; 6] = ["check", "build", "run", "repl", "help", "docs"];
    if let Some(idx) = args.iter().skip(1).position(|a| !a.starts_with('-')) {
        let idx = idx + 1;
        if !SUBCOMMANDS.contains(&args[idx].as_str()) {
            args.insert(idx, "run".to_string());
        }
    }
    args
}

/// The library the unit's scripts are checked against: what its host last wrote, or std alone
/// (saying why) when there's no manifest to read.
fn host_library(unit: &Unit) -> Library<()> {
    let found = api::Manifest::find(&unit.project);
    let (library, problem) = api::Manifest::library(found, std_alone);
    if let Some(problem) = problem {
        let warning = "warning".bright_yellow().bold();
        println!("{warning}: {problem}, so scripts are checked against std alone");
    }
    library
}

/// The standard library alone.
fn std_alone() -> Library<()> {
    vm::Vm::new().install_library(library::std)
}

fn resolve_path(path: Option<PathBuf>) -> PathBuf {
    path.unwrap_or_else(|| std::env::current_dir().expect("Cannot access the current directory!"))
}

fn format_duration(d: Duration) -> String {
    if d.as_micros() < 1000 {
        format!("{}µs", d.as_micros())
    } else if d.as_millis() < 1000 {
        format!("{}ms", d.as_millis())
    } else {
        format!("{:.2}s", d.as_secs_f32())
    }
}
