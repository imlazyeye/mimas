use api::Intrinsic;
use macros::native;
use rand::RngExt;
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

/// Returns the absolute value.
///
/// ```mimas
/// let a = (-7).abs(); // 7
/// ```
#[native]
fn abs<'gc>(n: i64) -> i64 {
    n.abs()
}

/// Returns the smaller of the number and `other`.
///
/// ```mimas
/// let a = 3.min(8); // 3
/// ```
#[native]
fn min<'gc>(n: i64, other: i64) -> i64 {
    n.min(other)
}

/// Returns the larger of the number and `other`.
///
/// ```mimas
/// let a = 3.max(8);        // 8
/// let hp = (5 - 9).max(0); // 0
/// ```
#[native]
fn max<'gc>(n: i64, other: i64) -> i64 {
    n.max(other)
}

/// Returns the number moved into the range from `low` to `high`, inclusive. A number already in
/// the range comes back unchanged.
///
/// ```mimas
/// let a = 15.clamp(0, 10);   // 10
/// let b = (-3).clamp(0, 10); // 0
/// let c = 4.clamp(0, 10);    // 4
/// ```
#[native]
fn clamp<'gc>(n: i64, low: i64, high: i64) -> i64 {
    n.clamp(low, high) // todo: fault when low > high
}

/// Returns the number written out in base 10, with a leading `-` if it's negative. An f-string
/// does the same inside a larger string.
///
/// ```mimas
/// let a = (-42).to_str(); // "-42"
/// let b = f"{7} lives";   // "7 lives"
/// ```
#[native]
fn to_str<'gc>(n: i64) -> String {
    n.to_string()
}

/// Returns a random `int` that is at least `0` and less than `len`. That makes
/// `int::random(xs.len())` a random index into `xs`.
///
/// `len` must be greater than `0`.
///
/// ```mimas
/// let roll = int::random(6) + 1; // 1 to 6
/// ```
#[native]
fn random<'gc>(len: i64) -> i64 {
    rand::rng().random_range(0..len)
}

/// Returns the number as a `float`. Arithmetic that mixes `int` and `float` converts on its own,
/// but passing an `int` where a `float` is expected doesn't, and needs this first:
///
/// ```mimas
/// fn half(x: float) -> float {
///     x / 2.0
/// }
///
/// let n = 3;
/// let a = half(n.to_float());  // 1.5
/// let b = n.to_float().sqrt(); // 1.7320508075688772
/// ```
#[native]
fn to_float<'gc>(n: i64) -> f64 {
    n as f64
}
