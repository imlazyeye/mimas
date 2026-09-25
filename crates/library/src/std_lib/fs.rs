use camino::{Utf8Path, Utf8PathBuf};
use itertools::Itertools;
use macros::native;
use vm::{Ctx, api::Api, conversion::Raisable};

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    let mut m = api.module("std::fs");
    m.add(read);
    m.add(write);
    m.add(list_dir);
    m.add(walk);
    m.add(remove);
    m.add(remove_dir);
    m.add(make_dir);
    m.add(cwd);
}

/// Returns the contents of the file at `path` as a string. Raises if the file can't be read, such
/// as when it doesn't exist or isn't valid UTF-8.
///
/// ```mimas
/// use std::fs;
///
/// let notes = fs::read("notes.txt")!;
/// let config = fs::read("config.txt") absolve |err| "";
/// ```
#[native]
fn read<'gc>(path: &str) -> Raisable<String> {
    std::fs::read_to_string(path).into()
}

// todo, we're returning a bool because Raisable<()> yields Raisable<Null> and an unwrap on it will
// panic!
/// Writes `contents` to the file at `path`, replacing anything the file held before, and returns
/// `true`. The file is created if it doesn't exist, but the directory it goes in has to exist
/// already (see [`make_dir`](#make_dir)). Raises if the file can't be written.
///
/// ```mimas
/// use std::fs;
///
/// fs::write("scores.txt", "ada 3\nbob 5\n")!;
/// ```
#[native]
fn write<'gc>(path: &str, contents: &str) -> Raisable<bool> {
    std::fs::write(path, contents).map(|_| true).into()
}

/// Deletes the file at `path` and returns `true`. Raises if it can't, including when `path` is a
/// directory ([`remove_dir`](#remove_dir) deletes those).
///
/// ```mimas
/// use std::fs;
///
/// fs::remove("scores.txt")!;
/// ```
#[native]
fn remove<'gc>(path: &str) -> Raisable<bool> {
    std::fs::remove_file(path).map(|_| true).into()
}

/// Deletes the directory at `path` and everything inside it, then returns `true`. Raises if it
/// can't, such as when the directory doesn't exist.
///
/// ```mimas
/// use std::fs;
///
/// fs::remove_dir("saves")!;
/// ```
#[native]
fn remove_dir<'gc>(path: &str) -> Raisable<bool> {
    std::fs::remove_dir_all(path).map(|_| true).into()
}

/// Creates a directory at `path`, along with any parent directories that are missing, and returns
/// `true`. A directory that already exists is left as it is. Raises if the directory can't be
/// created.
///
/// ```mimas
/// use std::fs;
///
/// fs::make_dir("saves/slot1")!;
/// fs::write("saves/slot1/state.txt", "level 3")!;
/// ```
#[native]
fn make_dir<'gc>(path: &str) -> Raisable<bool> {
    std::fs::create_dir_all(path).map(|_| true).into()
}

/// Returns the paths of the files and directories directly inside the directory at `path`. Each
/// path starts with `path` itself, and their order depends on the operating system. Raises if the
/// directory can't be read.
///
/// ```mimas
/// use std::fs;
///
/// for entry in fs::list_dir("saves")! {
///     print(entry); // saves/slot1, saves/slot2, ...
/// }
/// ```
///
/// [`walk`](#walk) goes into subdirectories as well.
#[native]
fn list_dir<'gc>(_ctx: Ctx<'gc>, path: &str) -> Raisable<Vec<String>> {
    let path = Utf8PathBuf::from(path);
    let read_dir = match path.read_dir_utf8() {
        Ok(it) => it,
        Err(e) => return Raisable::Raised(e.to_string()),
    };
    read_dir
        .map_ok(|v| v.path().to_string())
        .collect::<Result<Vec<_>, _>>()
        .into()
}

/// Returns the path of every file and directory under `path` at any depth, starting with `path`
/// itself. A directory's contents come right after it, and their order within the directory
/// depends on the operating system. Raises if anything under `path` can't be read.
///
/// ```mimas
/// use std::fs;
///
/// for path in fs::walk("assets")! {
///     if path.ends_with(".png") {
///         print(path);
///     }
/// }
/// ```
// todo, i want a walk_files version of this
#[native]
fn walk<'gc>(_ctx: Ctx<'gc>, path: &str) -> Raisable<Vec<String>> {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map_ok(|v| Utf8Path::from_path(v.path()).map(|s| s.to_string()))
        .collect::<Result<Vec<String>, _>>()
        .into()
}

/// Returns the absolute path of the current working directory. Raises if it can't be read, such
/// as when the directory has been deleted.
///
/// Relative paths given to `std::fs` start from this directory, which is where the program was
/// launched from. That isn't necessarily where the script is: `std::sys::file()` gives the
/// script's own path.
///
/// ```mimas
/// use std::fs;
///
/// print(fs::cwd()!); // /home/ada/projects/game
/// ```
#[native]
fn cwd<'gc>() -> Raisable<String> {
    std::env::current_dir()
        .map(|v| v.display().to_string())
        .into()
}
