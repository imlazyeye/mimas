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

#[native]
fn read<'gc>(path: &str) -> Raisable<String> {
    std::fs::read_to_string(path).into()
}

// todo, we're returning a bool because Raisable<()> yields Raisable<Null> and an unwrap on it will
// panic!
#[native]
fn write<'gc>(path: &str, output: &str) -> Raisable<bool> {
    std::fs::write(path, output).map(|_| true).into()
}

#[native]
fn remove<'gc>(path: &str) -> Raisable<bool> {
    std::fs::remove_file(path).map(|_| true).into()
}

#[native]
fn remove_dir<'gc>(path: &str) -> Raisable<bool> {
    std::fs::remove_dir_all(path).map(|_| true).into()
}

#[native]
fn make_dir<'gc>(path: &str) -> Raisable<bool> {
    std::fs::create_dir_all(path).map(|_| true).into()
}

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

// todo, i want a walk_files version of this
#[native]
fn walk<'gc>(_ctx: Ctx<'gc>, path: &str) -> Raisable<Vec<String>> {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map_ok(|v| Utf8Path::from_path(v.path()).map(|s| s.to_string()))
        .collect::<Result<Vec<String>, _>>()
        .into()
}

#[native]
fn cwd<'gc>() -> Raisable<String> {
    std::env::current_dir()
        .map(|v| v.display().to_string())
        .into()
}
