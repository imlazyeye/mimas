use macros::native;
use std::{io::Write, process::Stdio};
use vm::{api::Api, conversion::Raisable};

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    let mut m = api.module("std::process");
    m.add(run);
}

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
