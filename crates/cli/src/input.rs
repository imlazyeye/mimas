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
    /// Checks a script with the modules of its project, or every script in a directory. Scripts in
    /// a cargo package are checked against the API its host last wrote.
    Check {
        /// The path to the project directory to run on. Uses the current directory if not
        /// provided.
        #[clap(parse(from_os_str))]
        path: Option<PathBuf>,
    },
    /// Runs a script with the modules of its project, or a directory's only script.
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
    /// Builds but does not run. Scripts in a cargo package are checked against the API its host
    /// last wrote.
    Build {
        /// The path to the project directory to run on. Uses the current directory if not
        /// provided.
        #[clap(parse(from_os_str))]
        path: Option<PathBuf>,
    },
    /// Writes markdown documentation of what a Rust host gives its scripts, read from the manifest
    /// the host writes to `target/mimas` when it runs from there.
    Docs {
        /// The path to output the documentation to.
        #[clap(parse(from_os_str), required_unless_present = "mdbook")]
        output_path: Option<PathBuf>,

        /// The path to the manifest to build the documentation off of. Defaults to the newest one
        /// written by a binary of the cargo package you're in.
        #[clap(parse(from_os_str))]
        manifest_path: Option<PathBuf>,

        /// Includes the standard library, which is left out otherwise. Without a manifest, this
        /// documents the standard library alone.
        #[clap(long)]
        include_std: bool,

        /// Runs as an mdBook preprocessor instead, adding the pages under the chapter at this path
        /// (i.e. `api.md`) along with a table of them. The manifest is always the default one.
        #[clap(long, value_name = "CHAPTER")]
        mdbook: Option<String>,
    },
}
