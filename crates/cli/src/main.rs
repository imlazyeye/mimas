use api::Library;
use clap::Parser;
use colored::Colorize;
use num_format::{Locale, ToFormattedString};
use parse::Ast;
use solve::{Directory, Modules};
use std::{path::PathBuf, time::Duration};
use vm::conversion::Raisable;

mod ice;
mod input;
mod render;
mod unit;
pub use input::*;
use unit::Unit;

const ICE_EXIT_CODE: i32 = 101;
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
        Some(Commands::Docs {
            output_path,
            manifest_path,
            include_std: std,
            mdbook,
        }) => docs(output_path, manifest_path, std, mdbook, input.color),
        None => 0,
    };
    std::process::exit(status_code);
}

fn check(path: Option<PathBuf>, color: bool) -> i32 {
    let timer = std::time::Instant::now();
    let unit = Unit::new(&resolve_path(path));
    let library = host_library(&unit);
    let (directory, mut count) = match load(&unit, &library, color) {
        Ok(loaded) => loaded,
        Err(code) => return code,
    };
    for file in unit.scripts(&directory) {
        match solve(&directory.modules, file, color) {
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
    let (directory, mut count) = match load(&unit, &library, color) {
        Ok(loaded) => loaded,
        Err(code) => return code,
    };
    let mut scripts = vec![];
    for file in unit.scripts(&directory) {
        match solve(&directory.modules, file, color) {
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
        if let Err(code) = compile(&directory.modules, script, &library, disasm, dump_ir) {
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
    let directory = match load(&unit, &library, color) {
        Ok((directory, 0)) => directory,
        Ok(_) => return 1,
        Err(code) => return code,
    };
    let scripts = unit.scripts(&directory);
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
    let script = match solve(&directory.modules, file, color) {
        Ok(script) if script.errors.is_empty() => script,
        Ok(_) => return 1,
        Err(code) => return code,
    };
    let sources = script.sources.clone();
    let compiled = match compile(&directory.modules, script, &library, disasm, dump_ir) {
        Ok(compiled) => compiled,
        Err(code) => return code,
    };

    let mut resolved_args = Vec::with_capacity(script_args.len() + 1);
    resolved_args.push(file.0.to_string_lossy().into_owned());
    resolved_args.extend(script_args);
    vm.fixture::<library::ScriptArgs>().set(resolved_args);

    let result = ice::catch("execution", move || {
        vm.load_program(compiled);
        vm.set_sources(sources);
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

fn docs(
    output_path: Option<PathBuf>,
    manifest_path: Option<PathBuf>,
    std: bool,
    mdbook: Option<String>,
    color: bool,
) -> i32 {
    // mdbook asks `supports <renderer>` before running us, and markdown pages suit every renderer
    if mdbook.is_some() && output_path.is_some() {
        return 0;
    }

    let error = "error".bright_red().bold();
    let manifest = match manifest_path {
        Some(path) => api::Manifest::read(&path).map(Some),
        None => api::Manifest::find(&api::Project::of(&resolve_path(None))),
    };
    let host = match manifest {
        Ok(Some(manifest)) => manifest.library,
        Ok(None) if std => std_alone(),
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

    let files = [("docs.mim", DOCS_SCRIPT), ("mdbook.mim", MDBOOK_SCRIPT)];
    let result = vm::Vm::compile_files(&files, |api| {
        library::std(api);
        api.module("docs").add(display_ty);
    })
    .and_then(|mut vm| {
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

/// Loads the unit's project and prints the errors outside its scripts, counting them, or says
/// with which exit code that crashed.
fn load<'a>(
    unit: &'a Unit,
    library: &Library<()>,
    color: bool,
) -> Result<(Directory<'a>, usize), i32> {
    report_meta_errors(&unit.io_errors);
    let directory =
        ice::catch("check", || Directory::load(&unit.files, library)).map_err(|report| {
            report.emit();
            ICE_EXIT_CODE
        })?;

    for error in &directory.modules.errors {
        render::emit(error.as_ref(), color);
    }
    let count = unit.io_errors.len() + directory.modules.errors.len();
    return Ok((directory, count));

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
    let Modules {
        asts,
        sources,
        solver,
        ..
    } = script;
    let srcs: std::collections::HashMap<usize, std::sync::Arc<str>> = if disasm {
        sources
            .iter()
            .map(|(&id, ns)| (id, ns.inner().clone()))
            .collect()
    } else {
        std::collections::HashMap::new()
    };
    let intrinsics = library.intrinsics().clone();
    ice::catch("compilation", || {
        let mut ir = compile::Ir::new(solve::Resolutions::from(solver), intrinsics);
        ir.lower(modules.asts.iter().chain(&asts).flat_map(Ast::stmts));
        if dump_ir {
            println!("{ir}");
        }
        compile::Compiler::new()
            .with_disasm(disasm)
            .with_sources(srcs)
            .compile(ir)
    })
    .map_err(|report| {
        report.emit();
        ICE_EXIT_CODE
    })
}

// bare `mimas foo.mim` means `mimas run foo.mim`; inject `run` when the first
// positional isn't already a subcommand. `mimas` alone still falls through to help.
fn massage_args(mut args: Vec<String>) -> Vec<String> {
    const SUBCOMMANDS: [&str; 5] = ["check", "build", "run", "help", "docs"];
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
