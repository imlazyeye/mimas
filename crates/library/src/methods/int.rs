use api::Intrinsic;
use macros::native;
use rand::Rng;
use shared::Ty;
use vm::{RtErr, api::Api};

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    api.add_method(abs);
    api.add_method(min);
    api.add_method(max);
    api.add_method(clamp);
    api.add_method(to_str);
    let id = api.add_method(to_float);
    api.mark_intrinsic(id, Intrinsic::ToFloat);
    api.add_assoc(Ty::Int, random);
}

#[native]
fn abs<'gc>(n: i64) -> i64 {
    n.abs()
}

#[native]
fn min<'gc>(a: i64, b: i64) -> i64 {
    a.min(b)
}

#[native]
fn max<'gc>(a: i64, b: i64) -> i64 {
    a.max(b)
}

#[native]
fn clamp<'gc>(n: i64, lo: i64, hi: i64) -> Result<i64, RtErr> {
    if lo > hi {
        return Err(RtErr::InvalidArgument("clamp requires lo <= hi".into()));
    }
    Ok(n.clamp(lo, hi))
}

#[native]
fn to_str<'gc>(n: i64) -> String {
    n.to_string()
}

#[native]
fn random<'gc>(len: i64) -> i64 {
    rand::thread_rng().gen_range(0..len)
}

#[native]
fn to_float<'gc>(v: i64) -> f64 {
    v as f64
}
