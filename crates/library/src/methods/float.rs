use api::Intrinsic;
use macros::native;
use rand::RngExt;
use shared::Ty;
use vm::{RtErr, api::Api};

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    api.add_method(floor);
    api.add_method(to_str);
    api.add_method(ceil);
    api.add_method(round);
    api.add_method(to_int);
    api.add_method(abs);
    api.add_method(max);
    api.add_method(min);
    let id = api.add_method(sqrt);
    api.mark_intrinsic(id, Intrinsic::Sqrt);
    api.add_method(format);
    api.add_assoc(Ty::Float, random);
}

#[native]
fn floor<'gc>(n: f64) -> f64 {
    n.floor()
}

#[native]
fn round<'gc>(n: f64) -> f64 {
    n.round()
}

#[native]
fn ceil<'gc>(n: f64) -> f64 {
    n.ceil()
}

#[native]
fn to_int<'gc>(n: f64) -> i64 {
    n as i64
}

#[native]
fn abs<'gc>(n: f64) -> f64 {
    n.abs()
}

#[native]
fn max<'gc>(a: f64, b: f64) -> f64 {
    a.max(b)
}

#[native]
fn min<'gc>(a: f64, b: f64) -> f64 {
    a.min(b)
}

#[native]
fn sqrt<'gc>(n: f64) -> f64 {
    n.sqrt()
}

#[native]
fn format<'gc>(n: f64, places: usize) -> Result<String, RtErr> {
    // 1074 fractional digits covers the longest possible f64 expansion; anything past
    // that is zero padding, and a huge count would allocate it
    if places > 1074 {
        return Err(RtErr::InvalidArgument(
            "format places cannot exceed 1074".into(),
        ));
    }
    Ok(format!("{n:.places$}"))
}

#[native]
fn to_str<'gc>(n: f64) -> String {
    n.to_string()
}

#[native]
fn random<'gc>(len: f64) -> f64 {
    rand::rng().random_range(0.0..len)
}
