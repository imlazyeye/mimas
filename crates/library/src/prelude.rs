use macros::native;
use vm::{
    Ctx, RtErr, Val,
    anon::{self},
    api::Api,
    conversion::NeverReturn,
};

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    api.add(print);
    api.add(panic);
    api.add(todo);
    api.add(dbg);
}

#[native]
fn print<'gc>(ctx: Ctx<'gc>, msg: Val<'gc>) {
    println!("{}", ctx.to_string(msg));
}

#[native]
fn panic<'gc>(ctx: Ctx<'gc>, msg: Option<anon::T<'gc>>) -> Result<NeverReturn, RtErr> {
    Err(RtErr::Custom(
        msg.map(|msg| format!("panic: {}", ctx.to_string(msg.0)))
            .unwrap_or_else(|| "explicit panic".into()),
    ))
}

#[native]
fn todo<'gc>(ctx: Ctx<'gc>, msg: Option<anon::T<'gc>>) -> Result<NeverReturn, RtErr> {
    Err(match msg {
        Some(m) => RtErr::Custom(format!("todo: {}", ctx.to_string(m.0))),
        None => RtErr::Custom("todo".into()),
    })
}

#[native]
fn dbg<'gc>(ctx: Ctx<'gc>, val: anon::T<'gc>) -> anon::T<'gc> {
    println!("dbg value: {}", ctx.display(val.0));
    val
}
