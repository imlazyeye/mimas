use std::{cell::RefCell, io::Read as _};

use macros::native;
use vm::{
    Ctx,
    api::Api,
    conversion::{NeverReturn, Raisable},
};

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    let mut m = api.module("std::sys");
    m.add(arg);
    m.add(exit);
    m.add(stdin);
    let file = m.add(file);
    api.mark_intrinsic(file, api::Intrinsic::File);
}

/// Returns the command-line argument at `index`, or `null` if there isn't one. When the script is
/// run with the `mimas` command, index `0` is the script's path as it was typed, and the arguments
/// after `--` start at index `1`.
///
/// ```mimas
/// use std::sys;
///
/// // run as `mimas tool.mim -- input.txt`
/// let path = sys::arg(1) ?? "default.txt"; // "input.txt"
/// ```
#[native]
fn arg<'gc>(ctx: Ctx<'gc>, index: usize) -> Option<String> {
    ctx.fixture::<ScriptArgs>().get(index)
}

/// Ends the program immediately with the exit code `code`. Nothing after the call runs.
///
/// ```mimas
/// use std::sys;
///
/// if sys::arg(1) == null {
///     print("usage: tool <file>");
///     sys::exit(1);
/// }
/// ```
///
/// `exit` ends the whole process. When mimas is embedded in a larger program, that program exits
/// as well.
#[native]
fn exit<'gc>(code: i32) -> NeverReturn {
    std::process::exit(code)
}

/// Reads standard input until it ends and returns all of it as one string. Raises if the input
/// can't be read or isn't valid UTF-8.
///
/// Input piped in from another program ends when that program finishes. In a terminal, it ends
/// when you press Ctrl+D (Ctrl+Z then Enter on Windows).
///
/// ```mimas
/// use std::sys;
///
/// // run as `echo "a b c" | mimas count.mim`
/// let words = sys::stdin()!.trim().split(" ");
/// print(words.len()); // 3
/// ```
#[native]
fn stdin<'gc>() -> Raisable<String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map(|_| buf)
        .into()
}

/// Returns the absolute path of the source file that contains the call. For a script compiled
/// from a string instead of a file, it's the name the host gave that source.
///
/// Use it to find files stored next to the script (relative paths in `std::fs` start from the
/// working directory instead).
///
/// ```mimas
/// use std::sys;
///
/// print(sys::file()); // /home/ada/projects/game/main.mim
/// ```
#[native]
fn file() -> String {
    "<unknown>".into()
}

/// The arguments `std::sys::arg` reads. The `mimas` command fills it with the script's path
/// followed by its arguments. A host that embeds mimas can set its own before running a script,
/// with `vm.fixture::<ScriptArgs>().set(args)`.
#[derive(Default)]
pub struct ScriptArgs(RefCell<Vec<String>>);

impl ScriptArgs {
    /// Replaces the arguments. Index `0` is conventionally the script's path.
    pub fn set(&self, args: Vec<String>) {
        *self.0.borrow_mut() = args;
    }

    fn get(&self, idx: usize) -> Option<String> {
        self.0.borrow().get(idx).cloned()
    }
}
