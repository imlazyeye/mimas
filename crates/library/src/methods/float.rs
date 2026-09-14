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
    api.add_method(clamp);
    api.add_method(signum);
    api.add_method(sin);
    api.add_method(cos);
    api.add_method(tan);
    api.add_method(asin);
    api.add_method(acos);
    api.add_method(atan);
    api.add_method(atan2);
    api.add_method(pow);
    api.add_method(exp);
    api.add_method(ln);
    api.add_assoc(Ty::Float, random);
}

#[native]
fn clamp(n: f64, low: f64, high: f64) -> f64 {
    n.clamp(low, high)
}

#[native]
fn signum(n: f64) -> f64 {
    n.signum()
}

#[native]
fn sin(radians: f64) -> f64 {
    radians.sin()
}

#[native]
fn cos(radians: f64) -> f64 {
    radians.cos()
}

#[native]
fn tan(radians: f64) -> f64 {
    radians.tan()
}

#[native]
fn asin(n: f64) -> f64 {
    n.asin()
}

#[native]
fn acos(n: f64) -> f64 {
    n.acos()
}

#[native]
fn atan(n: f64) -> f64 {
    n.atan()
}

#[native]
fn atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

#[native]
fn pow(n: f64, exponent: f64) -> f64 {
    n.powf(exponent)
}

#[native]
fn exp(n: f64) -> f64 {
    n.exp()
}

#[native]
fn ln(n: f64) -> f64 {
    n.ln()
}

#[native]
fn floor(n: f64) -> f64 {
    n.floor()
}

#[native]
fn round(n: f64) -> f64 {
    n.round()
}

#[native]
fn ceil(n: f64) -> f64 {
    n.ceil()
}

#[native]
fn to_int(n: f64) -> i64 {
    n as i64
}

#[native]
fn abs(n: f64) -> f64 {
    n.abs()
}

#[native]
fn max(a: f64, b: f64) -> f64 {
    a.max(b)
}

#[native]
fn min(a: f64, b: f64) -> f64 {
    a.min(b)
}

#[native]
fn sqrt(n: f64) -> f64 {
    n.sqrt()
}

#[native]
fn format(n: f64, places: usize) -> Result<String, RtErr> {
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
fn to_str(n: f64) -> String {
    n.to_string()
}

#[native]
fn random(len: f64) -> f64 {
    rand::rng().random_range(0.0..len)
}
