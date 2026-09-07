use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "The mimas compiler and runtime: check, build, and run .mim scripts."
)]
#[clap(setting(clap::AppSettings::ArgRequiredElseHelp))]
pub struct Cli {
    /// Force color output instead of deferring.
    #[clap(long, global = true)]
    pub color: bool,
    /// If provided the total duration mimas ran for will be printed at the end.
    #[clap(long, global = true)]
    pub time: bool,
    /// Dump the compiled bytecode per body (op count, register count, move count, disassembly).
    #[clap(long, global = true)]
    pub dump_bytes: bool,
    /// Dump the SSA IR (blocks, phis, instructions) before codegen.
    #[clap(long = "dump-ir", global = true)]
    pub dump_ir: bool,
    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Checks the validity of the provided directory of mimas code.
    Check {
        /// The path to the project directory to run on. Uses the current directory if not
        /// provided.
        #[clap(parse(from_os_str))]
        path: Option<PathBuf>,
    },
    /// Runs a mimas script, or a project directory containing a main.mim.
    Run {
        /// The path to the project directory to run on. Uses the current directory if not
        /// provided.
        #[clap(parse(from_os_str))]
        path: Option<PathBuf>,

        /// Arguments forwarded to the script, accessible as `std::sys::arg(1..)`.
        /// `std::sys::arg(0)` is always the script path.
        /// Pass these after a `--` separator: `mimas run script.mim -- foo bar`.
        #[clap(last = true)]
        script_args: Vec<String>,
    },
    /// Builds but does not run
    Build {
        /// The path to the project directory to run on. Uses the current directory if not
        /// provided.
        #[clap(parse(from_os_str))]
        path: Option<PathBuf>,
    },
}
