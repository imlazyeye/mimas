use macros::native;
use std::{io::Write, process::Stdio};
use vm::{api::Api, conversion::Raisable};

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    let mut m = api.module("std::process");
    m.add(run);
    m.add(run_attached);
}

/// Runs the program `cmd` with `args`, waits for it to finish, and returns what it wrote to
/// standard output. When `stdin` is given, it's written to the program's standard input.
///
/// `cmd` is looked up on the `PATH`, but there's no shell in between. Each argument is passed to
/// the program exactly as written. The output keeps its trailing newline, which `trim` removes.
///
/// Raises if the program can't be started or its output isn't valid UTF-8. A non-zero exit status
/// raises as well, with the program's standard error in the message.
///
/// ```mimas
/// use std::process;
///
/// let greeting = process::run("echo", ["hello"])!.trim(); // "hello"
/// let sorted = process::run("sort", [], "b\na\n")!;       // "a\nb\n"
/// ```
#[native]
fn run<'gc>(cmd: &str, args: Vec<&str>, stdin: Option<&str>) -> Raisable<String> {
    let inner = || -> Result<String, String> {
        let mut command = std::process::Command::new(cmd);
        command.args(args);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        if stdin.is_some() {
            command.stdin(Stdio::piped());
        }

        let mut child = command.spawn().map_err(|e| format!("spawn {cmd:?}: {e}"))?;

        if let Some(input) = stdin {
            let mut sin = child
                .stdin
                .take()
                .expect("stdin pipe requested but missing");
            sin.write_all(input.as_bytes())
                .map_err(|e| format!("stdin write: {e}"))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("wait_with_output: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "{cmd:?} exited with {}: {}",
                output.status,
                stderr.trim()
            ));
        }

        String::from_utf8(output.stdout).map_err(|e| format!("non-utf8 stdout: {e}"))
    };

    inner().into()
}

/// Runs the program `cmd` with `args`, waits for it to finish, and returns its exit code.
///
/// Unlike [`run`](#run), the program uses the script's own standard input and output. Anything it
/// prints appears directly (usually in the terminal) instead of coming back as a string, and a
/// non-zero exit code is returned like any other. Raises if the program can't be started or is
/// stopped by a signal.
///
/// ```mimas
/// use std::process;
///
/// let code = process::run_attached("cargo", ["test"])!;
/// if code != 0 {
///     print("tests failed");
/// }
/// ```
#[native]
fn run_attached<'gc>(cmd: &str, args: Vec<&str>) -> Raisable<i64> {
    let inner = || -> Result<i64, String> {
        let status = std::process::Command::new(cmd)
            .args(args)
            .status()
            .map_err(|e| format!("spawn {cmd:?}: {e}"))?;
        status
            .code()
            .map(i64::from)
            .ok_or_else(|| format!("{cmd:?} was killed by a signal"))
    };

    inner().into()
}
