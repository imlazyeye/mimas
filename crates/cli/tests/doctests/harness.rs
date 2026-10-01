use std::{
    fs::{self, File},
    process::{Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

macro_rules! doctest {
    ($name:ident, $location:literal, ignore, $source:literal) => {
        #[test]
        #[ignore]
        fn $name() {
            $crate::harness::run(
                stringify!($name),
                $location,
                $crate::harness::Expect::Runs,
                $source,
            );
        }
    };
    ($name:ident, $location:literal, $expect:expr, $source:literal) => {
        #[test]
        fn $name() {
            $crate::harness::run(stringify!($name), $location, $expect, $source);
        }
    };
}

/// What a block has to do, where the lines are the block's own, counted from 1.
pub enum Expect {
    Runs,
    Checks,
    CompileError(&'static [usize]),
    RuntimeError(&'static [usize]),
}

/// Runs `source` as its own `mimas` process, so a block that exits or writes files is isolated.
pub fn run(name: &str, location: &str, expect: Expect, source: &str) {
    // outside the repo, so a block that runs cargo can't find the workspace
    let dir = std::env::temp_dir().join("mimas-doctests").join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("main.mim"), source).unwrap();
    let mimas = |command: &str| -> (ExitStatus, String) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mimas"))
            .args([command, "main.mim"])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(File::create(dir.join("stderr")).unwrap())
            .spawn()
            .unwrap();
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if started.elapsed() > Duration::from_secs(10) {
                child.kill().unwrap();
                panic!("{location} timed out");
            }
            thread::sleep(Duration::from_millis(5));
        };
        (status, fs::read_to_string(dir.join("stderr")).unwrap())
    };

    // a module file can't run, so it only has to check
    let checks =
        matches!(expect, Expect::Checks | Expect::CompileError(_)) || parse::lex::is_module(source);
    if let Expect::RuntimeError(_) = expect {
        let (status, stderr) = mimas("check");
        assert!(status.success(), "{location} should check\n{stderr}");
    }
    let (status, stderr) = mimas(if checks { "check" } else { "run" });
    let (Expect::CompileError(marked) | Expect::RuntimeError(marked)) = expect else {
        assert!(status.success(), "{location} {status}\n{stderr}");
        return;
    };
    // each error's own span sits right under its title, below that can come related spans
    let lines: Vec<&str> = stderr.lines().collect();
    let failed_on: Vec<usize> = lines
        .windows(2)
        .filter(|pair| pair[0].trim_start().starts_with("x "))
        .filter_map(|pair| {
            pair[1]
                .split("main.mim:")
                .nth(1)?
                .split(':')
                .next()?
                .parse()
                .ok()
        })
        .collect();
    assert!(
        !status.success()
            && !failed_on.is_empty()
            && failed_on.iter().all(|line| marked.contains(line)),
        "{location} should fail on line {marked:?} of the block, not {failed_on:?}\n{stderr}"
    );
}
