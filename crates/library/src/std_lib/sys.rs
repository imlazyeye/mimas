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
}

#[native]
fn arg<'gc>(ctx: Ctx<'gc>, index: usize) -> Option<String> {
    ctx.fixture::<ScriptArgs>().get(index)
}

#[native]
fn exit<'gc>(code: i32) -> NeverReturn {
    std::process::exit(code)
}

#[native]
fn stdin<'gc>() -> Raisable<String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map(|_| buf)
        .into()
}

#[derive(Default)]
pub struct ScriptArgs(RefCell<Vec<String>>);

impl ScriptArgs {
    pub fn set(&self, args: Vec<String>) {
        *self.0.borrow_mut() = args;
    }

    fn get(&self, idx: usize) -> Option<String> {
        self.0.borrow().get(idx).cloned()
    }
}
