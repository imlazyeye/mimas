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

/// Returns the number moved into the range from `low` to `high`, inclusive. A number already in
/// the range comes back unchanged.
///
/// ```mimas
/// let a = 1.5.clamp(0.0, 1.0);    // 1.0
/// let b = (-0.2).clamp(0.0, 1.0); // 0.0
/// ```
#[native]
fn clamp(n: f64, low: f64, high: f64) -> f64 {
    n.clamp(low, high) // todo: fault when low > high
}

/// Returns the sign of the number as `1.0` or `-1.0`. Zero counts as positive (`0.0.signum()` is
/// `1.0`), unless it's `-0.0`.
///
/// ```mimas
/// let a = (-3.2).signum(); // -1.0
/// let b = 5.0.signum();    // 1.0
/// ```
#[native]
fn signum(n: f64) -> f64 {
    n.signum()
}

/// Returns the sine of an angle in radians. To work in degrees, multiply by `PI / 180.0` first.
///
/// ```mimas
/// use std::math::PI;
///
/// let a = (PI / 2.0).sin();          // 1.0
/// let b = (90.0 * PI / 180.0).sin(); // 1.0
/// ```
#[native]
fn sin(radians: f64) -> f64 {
    radians.sin()
}

/// Returns the cosine of an angle in radians.
///
/// ```mimas
/// use std::math::PI;
///
/// let a = PI.cos(); // -1.0
/// ```
#[native]
fn cos(radians: f64) -> f64 {
    radians.cos()
}

/// Returns the tangent of an angle in radians.
///
/// ```mimas
/// let a = 0.0.tan(); // 0.0
/// ```
#[native]
fn tan(radians: f64) -> f64 {
    radians.tan()
}

/// Returns the arcsine in radians, between `-PI / 2.0` and `PI / 2.0`. A number outside `-1.0` to
/// `1.0` gives `NaN`.
///
/// ```mimas
/// let a = 1.0.asin(); // 1.5707963267948966 (PI / 2.0)
/// ```
#[native]
fn asin(n: f64) -> f64 {
    n.asin()
}

/// Returns the arccosine in radians, between `0.0` and `PI`. A number outside `-1.0` to `1.0`
/// gives `NaN`.
///
/// ```mimas
/// let a = 1.0.acos(); // 0.0
/// ```
#[native]
fn acos(n: f64) -> f64 {
    n.acos()
}

/// Returns the arctangent in radians, between `-PI / 2.0` and `PI / 2.0`. To get an angle from a
/// point or direction, [`atan2`](#atan2) is usually the better choice.
///
/// ```mimas
/// let a = 1.0.atan(); // 0.7853981633974483 (PI / 4.0)
/// ```
#[native]
fn atan(n: f64) -> f64 {
    n.atan()
}

/// Returns the angle in radians from the positive x axis to the point (`x`, `y`), where `y` is the
/// number this is called on. The result is between `-PI` and `PI`.
///
/// Unlike `(y / x).atan()`, it uses the signs of both coordinates to find the right quadrant, and
/// an `x` of `0.0` is fine.
///
/// ```mimas
/// let a = 1.0.atan2(1.0);  // 0.7853981633974483 (PI / 4.0)
/// let b = 1.0.atan2(-1.0); // 2.356194490192345 (3.0 * PI / 4.0)
/// ```
#[native]
fn atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

/// Returns the number raised to the power `exponent`.
///
/// ```mimas
/// let a = 2.0.pow(10.0); // 1024.0
/// let b = 9.0.pow(0.5);  // 3.0
/// ```
#[native]
fn pow(n: f64, exponent: f64) -> f64 {
    n.powf(exponent)
}

/// Returns `E` raised to the power of the number.
///
/// ```mimas
/// let a = 1.0.exp(); // 2.718281828459045
/// ```
#[native]
fn exp(n: f64) -> f64 {
    n.exp()
}

/// Returns the natural logarithm (base `E`). `0.0` gives negative infinity, and a negative number
/// gives `NaN`.
///
/// ```mimas
/// use std::math::E;
///
/// let a = E.ln();   // 1.0
/// let b = 1.0.ln(); // 0.0
/// ```
#[native]
fn ln(n: f64) -> f64 {
    n.ln()
}

/// Returns the largest whole number less than or equal to the number, as a `float`.
///
/// ```mimas
/// let a = 2.7.floor();    // 2.0
/// let b = (-2.7).floor(); // -3.0
/// ```
#[native]
fn floor(n: f64) -> f64 {
    n.floor()
}

/// Returns the nearest whole number, as a `float`. A number exactly halfway between two whole
/// numbers rounds away from zero.
///
/// ```mimas
/// let a = 2.4.round();    // 2.0
/// let b = 2.5.round();    // 3.0
/// let c = (-2.5).round(); // -3.0
/// ```
#[native]
fn round(n: f64) -> f64 {
    n.round()
}

/// Returns the smallest whole number greater than or equal to the number, as a `float`.
///
/// ```mimas
/// let a = 2.2.ceil();    // 3.0
/// let b = (-2.2).ceil(); // -2.0
/// ```
#[native]
fn ceil(n: f64) -> f64 {
    n.ceil()
}

/// Returns the number as an `int`, dropping anything after the decimal point. Round first with
/// [`round`](#round) to get the nearest `int` instead.
///
/// ```mimas
/// let a = 2.9.to_int();         // 2
/// let b = (-2.9).to_int();      // -2
/// let c = 2.9.round().to_int(); // 3
/// ```
#[native]
fn to_int(n: f64) -> i64 {
    n as i64
}

/// Returns the absolute value.
///
/// ```mimas
/// let a = (-1.5).abs(); // 1.5
/// ```
#[native]
fn abs(n: f64) -> f64 {
    n.abs()
}

/// Returns the larger of the number and `other`.
///
/// ```mimas
/// let a = 1.5.max(2.5); // 2.5
/// ```
#[native]
fn max(n: f64, other: f64) -> f64 {
    n.max(other)
}

/// Returns the smaller of the number and `other`.
///
/// ```mimas
/// let a = 1.5.min(2.5); // 1.5
/// ```
#[native]
fn min(n: f64, other: f64) -> f64 {
    n.min(other)
}

/// Returns the square root. A negative number gives `NaN`.
///
/// ```mimas
/// let a = 9.0.sqrt(); // 3.0
/// ```
#[native]
fn sqrt(n: f64) -> f64 {
    n.sqrt()
}

/// Returns the number as a string with exactly `places` digits after the decimal point. The last
/// digit is rounded, with a tie going to the even digit (`2.5.format(0)` is `"2"`, but
/// `3.5.format(0)` is `"4"`).
///
/// ```mimas
/// let a = 3.14159.format(2); // "3.14"
/// let b = 2.0.format(3);     // "2.000"
/// ```
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

/// Returns the number as a string, using the fewest digits that still read back as the same
/// `float`. A whole number has no `.0`. For a fixed number of decimal places, use
/// [`format`](#format).
///
/// ```mimas
/// let a = 0.1.to_str();         // "0.1"
/// let b = 2.0.to_str();         // "2"
/// let c = (0.1 + 0.2).to_str(); // "0.30000000000000004"
/// ```
#[native]
fn to_str(n: f64) -> String {
    n.to_string()
}

/// Returns a random `float` that is at least `0.0` and less than `len`.
///
/// `len` must be greater than `0.0`.
///
/// ```mimas
/// if float::random(1.0) < 0.25 {
///     print("a one in four chance");
/// }
/// ```
#[native]
fn random(len: f64) -> f64 {
    rand::rng().random_range(0.0..len)
}
