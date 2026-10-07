use macros::native;
use shared::{Literal, Ty};
use vm::{
    RtErr,
    api::Api,
    glam::{I64Vec2, I64Vec3, Quat, Vec2, Vec3},
};

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    let mut m = api.module("std::math");
    m.constant(
        "PI",
        Ty::Float,
        Literal::Float(std::f64::consts::PI),
        "The ratio of a circle's circumference to its diameter, about `3.14159`. An angle of `PI` \
         radians is half a turn.",
    );
    m.constant(
        "TAU",
        Ty::Float,
        Literal::Float(std::f64::consts::TAU),
        "A full turn in radians, equal to `2.0 * PI` (about `6.28319`).",
    );
    m.constant(
        "E",
        Ty::Float,
        Literal::Float(std::f64::consts::E),
        "Euler's number, the base of the natural logarithm, about `2.71828`.",
    );
    m.add_adt::<Vec2>();
    m.add_adt::<Vec3>();
    m.add_adt::<Quat>();
    m.add_adt::<I64Vec2>();
    m.add_adt::<I64Vec3>();
    m.add(vec2);
    m.add(vec3);
    m.add(ivec2);
    m.add(ivec3);
    api.add_assoc_of::<Vec2, _, _>("new", vec2_new);
    api.add_assoc_of::<Vec2, _, _>("from_angle", vec2_from_angle);
    api.add_assoc_of::<Vec2, _, _>("zero", vec2_zero);
    api.add_method_named("length", vec2_length);
    api.add_method_named("length_squared", vec2_length_squared);
    api.add_method_named("normalize", vec2_normalize);
    api.add_method_named("add", vec2_add);
    api.add_method_named("sub", vec2_sub);
    api.add_method_named("scale", vec2_scale);
    api.add_method_named("dot", vec2_dot);
    api.add_method_named("distance", vec2_distance);
    api.add_method_named("distance_squared", vec2_distance_squared);
    api.add_method_named("rotate", vec2_rotate);
    api.add_method_named("perp", vec2_perp);
    api.add_method_named("to_ivec2", vec2_to_ivec2);
    api.add_method_named("angle", vec2_angle);
    api.add_method_named("lerp", vec2_lerp);
    api.add_method_named("length", vec3_length);
    api.add_method_named("normalize", vec3_normalize);
    api.add_assoc_of::<Quat, _, _>("from_rotation_z", quat_from_rotation_z);
    api.add_method_named("rotate_z", quat_rotate_z);
    api.add_assoc_of::<I64Vec2, _, _>("new", ivec2_new);
    api.add_assoc_of::<I64Vec2, _, _>("zero", ivec2_zero);
    api.add_assoc_of::<I64Vec2, _, _>("from_index", ivec2_from_index);
    api.add_method_named("add", ivec2_add);
    api.add_method_named("sub", ivec2_sub);
    api.add_method_named("scale", ivec2_scale);
    api.add_method_named("dot", ivec2_dot);
    api.add_method_named("length_squared", ivec2_length_squared);
    api.add_method_named("distance_squared", ivec2_distance_squared);
    api.add_method_named("to_vec2", ivec2_to_vec2);
    api.add_method_named("to_index", ivec2_to_index);
    api.add_assoc_of::<I64Vec3, _, _>("new", ivec3_new);
    api.add_method_named("add", ivec3_add);
    api.add_method_named("sub", ivec3_sub);
    api.add_method_named("scale", ivec3_scale);
}

/// Creates a `Vec2` from its `x` and `y` components, as a shorter way to write `Vec2::new`.
///
/// ```mimas
/// use std::math::vec2;
///
/// let v = vec2(3.0, 4.0);
/// let y = v.y; // 4.0
/// ```
#[native]
fn vec2(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

/// Creates a `Vec3` from its `x`, `y` and `z` components.
///
/// ```mimas
/// use std::math::vec3;
///
/// let v = vec3(2.0, 3.0, 6.0);
/// let n = v.length(); // 7.0
/// ```
#[native]
fn vec3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

/// Creates an `IVec2` from its `x` and `y` components, as a shorter way to write `IVec2::new`.
///
/// ```mimas
/// use std::math::ivec2;
///
/// let v = ivec2(3, 4);
/// let y = v.y; // 4
/// ```
#[native]
fn ivec2(x: i64, y: i64) -> I64Vec2 {
    I64Vec2::new(x, y)
}

/// Creates an `IVec3` from its `x`, `y` and `z` components, as a shorter way to write
/// `IVec3::new`.
///
/// ```mimas
/// use std::math::ivec3;
///
/// let v = ivec3(1, 2, 3);
/// let z = v.z; // 3
/// ```
#[native]
fn ivec3(x: i64, y: i64, z: i64) -> I64Vec3 {
    I64Vec3::new(x, y, z)
}

/// Creates a vector from its `x` and `y` components. A struct literal such as
/// `Vec2 { x = 3.0, y = 4.0 }` does the same.
///
/// ```mimas
/// use std::math::Vec2;
///
/// let v = Vec2::new(3.0, 4.0);
/// let y = v.y; // 4.0
/// ```
#[native]
fn vec2_new(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

/// Creates a vector of length `1.0` pointing at an angle of `radians`, measured from the positive
/// x axis toward the positive y axis. Its components are the angle's cosine and sine.
///
/// ```mimas
/// use std::math::Vec2;
///
/// let right = Vec2::from_angle(0.0);
/// let x = right.x; // 1.0
/// ```
#[native]
fn vec2_from_angle(radians: f32) -> Vec2 {
    Vec2::from_angle(radians)
}

/// Creates the vector whose components are both `0.0`.
///
/// ```mimas
/// use std::math::Vec2;
///
/// let velocity = Vec2::zero();
/// let x = velocity.x; // 0.0
/// ```
#[native]
fn vec2_zero() -> Vec2 {
    Vec2::ZERO
}

#[native]
fn vec2_length(v: Vec2) -> f32 {
    v.length()
}

/// Returns the vector's length multiplied by itself, which skips the square root that `length`
/// takes. Comparing it against a squared distance tells which of two lengths is longer.
///
/// ```mimas
/// use std::math::Vec2;
///
/// let n = Vec2::new(3.0, 4.0).length_squared(); // 25.0
/// ```
#[native]
fn vec2_length_squared(v: Vec2) -> f32 {
    v.length_squared()
}

#[native]
fn vec2_normalize(v: Vec2) -> Vec2 {
    v.normalize_or_zero()
}

/// Returns the sum of the two vectors, adding them component by component.
///
/// ```mimas
/// use std::math::Vec2;
///
/// let v = Vec2::new(3.0, 4.0).add(Vec2::new(1.0, 1.0));
/// let x = v.x; // 4.0
/// ```
#[native]
fn vec2_add(v: Vec2, other: Vec2) -> Vec2 {
    v + other
}

/// Returns this vector minus `other`, component by component. `b.sub(a)` is the vector that
/// points from `a` to `b`.
///
/// ```mimas
/// use std::math::Vec2;
///
/// let player = Vec2::new(1.0, 1.0);
/// let enemy = Vec2::new(4.0, 5.0);
/// let offset = enemy.sub(player);
/// let x = offset.x; // 3.0
/// ```
#[native]
fn vec2_sub(v: Vec2, other: Vec2) -> Vec2 {
    v - other
}

/// Returns the vector with both components multiplied by `by`.
///
/// ```mimas
/// use std::math::Vec2;
///
/// let half = Vec2::new(3.0, 4.0).scale(0.5);
/// let x = half.x; // 1.5
/// ```
#[native]
fn vec2_scale(v: Vec2, by: f32) -> Vec2 {
    v * by
}

/// Returns the dot product, `x * other.x + y * other.y`. For two vectors of length `1.0` it's the
/// cosine of the angle between them, and a result of `0.0` means they're perpendicular.
///
/// ```mimas
/// use std::math::Vec2;
///
/// let a = Vec2::new(3.0, 4.0).dot(Vec2::new(1.0, 0.0)); // 3.0
/// ```
#[native]
fn vec2_dot(v: Vec2, other: Vec2) -> f32 {
    v.dot(other)
}

/// Returns the straight-line distance between the two points.
///
/// ```mimas
/// use std::math::Vec2;
///
/// let d = Vec2::new(0.0, 0.0).distance(Vec2::new(3.0, 4.0)); // 5.0
/// ```
#[native]
fn vec2_distance(v: Vec2, other: Vec2) -> f32 {
    v.distance(other)
}

/// Returns the distance between the two points multiplied by itself, which skips the square root
/// that `distance` takes.
///
/// ```mimas
/// use std::math::Vec2;
///
/// let player = Vec2::new(0.0, 0.0);
/// let enemy = Vec2::new(3.0, 4.0);
/// let close = player.distance_squared(enemy) < 6.0 * 6.0; // true
/// ```
#[native]
fn vec2_distance_squared(v: Vec2, other: Vec2) -> f32 {
    v.distance_squared(other)
}

/// Returns the vector turned by `radians` around the origin, from the positive x axis toward the
/// positive y axis. Its length doesn't change.
///
/// ```mimas
/// use std::math::{PI, Vec2};
///
/// let up = Vec2::new(1.0, 0.0).rotate(PI / 2.0);
/// let y = up.y; // 1.0
/// ```
#[native]
fn vec2_rotate(v: Vec2, radians: f32) -> Vec2 {
    Vec2::from_angle(radians).rotate(v)
}

/// Returns the vector turned a quarter turn, from the positive x axis toward the positive y axis.
/// It's `Vec2::new(-y, x)`, which is perpendicular to the vector and as long.
///
/// ```mimas
/// use std::math::Vec2;
///
/// let side = Vec2::new(1.0, 0.0).perp();
/// let y = side.y; // 1.0
/// ```
#[native]
fn vec2_perp(v: Vec2) -> Vec2 {
    v.perp()
}

/// Returns the vector as an `IVec2`, with each component truncated toward zero the way
/// `float.to_int` does. It's the step from a position to the pixel it's in.
///
/// ```mimas
/// use std::math::Vec2;
///
/// let pixel = Vec2::new(3.7, -1.2).to_ivec2();
/// let y = pixel.y; // -1
/// ```
#[native]
fn vec2_to_ivec2(v: Vec2) -> I64Vec2 {
    v.as_i64vec2()
}

/// Returns the angle of the vector in radians, measured from the positive x axis toward the
/// positive y axis. The result is between `-PI` and `PI`. `Vec2::from_angle` goes the other way.
///
/// ```mimas
/// use std::math::Vec2;
///
/// let a = Vec2::new(1.0, 0.0).angle(); // 0.0
/// ```
#[native]
fn vec2_angle(v: Vec2) -> f32 {
    v.to_angle()
}

/// Returns the point that is `t` of the way from this vector to `other`, so `0.5` gives the
/// midpoint. `t` isn't limited to `0.0` through `1.0`: `1.5` goes past `other` by half the
/// distance again.
///
/// ```mimas
/// use std::math::Vec2;
///
/// let start = Vec2::new(0.0, 0.0);
/// let end = Vec2::new(10.0, 20.0);
/// let quarter = start.lerp(end, 0.25);
/// let y = quarter.y; // 5.0
/// ```
#[native]
fn vec2_lerp(v: Vec2, other: Vec2, t: f32) -> Vec2 {
    v.lerp(other, t)
}

#[native]
fn vec3_length(v: Vec3) -> f32 {
    v.length()
}

#[native]
fn vec3_normalize(v: Vec3) -> Vec3 {
    v.normalize_or_zero()
}

/// Creates a rotation of `radians` around the z axis. For 2D work, this is a turn within the x-y
/// plane.
///
/// ```mimas
/// use std::math::{PI, Quat};
///
/// let quarter_turn = Quat::from_rotation_z(PI / 2.0);
/// ```
#[native]
fn quat_from_rotation_z(radians: f32) -> Quat {
    Quat::from_rotation_z(radians)
}

/// Returns the rotation turned a further `radians` around its own z axis.
///
/// ```mimas
/// use std::math::{PI, Quat};
///
/// let facing = Quat::from_rotation_z(0.0);
/// let turned = facing.rotate_z(PI / 2.0); // a quarter turn
/// ```
#[native]
fn quat_rotate_z(q: Quat, radians: f32) -> Quat {
    q * Quat::from_rotation_z(radians)
}

/// Creates a vector from its `x` and `y` components. A struct literal such as
/// `IVec2 { x = 3, y = 4 }` does the same.
///
/// ```mimas
/// use std::math::IVec2;
///
/// let v = IVec2::new(3, 4);
/// let y = v.y; // 4
/// ```
#[native]
fn ivec2_new(x: i64, y: i64) -> I64Vec2 {
    I64Vec2::new(x, y)
}

/// Creates the vector whose components are both `0`.
///
/// ```mimas
/// use std::math::IVec2;
///
/// let origin = IVec2::zero();
/// let x = origin.x; // 0
/// ```
#[native]
fn ivec2_zero() -> I64Vec2 {
    I64Vec2::ZERO
}

/// Returns the sum of the two vectors, adding them component by component. Like `int` math, a
/// result too large for an `int` is a runtime error, and that holds for every `IVec2` and `IVec3`
/// method.
///
/// ```mimas
/// use std::math::IVec2;
///
/// let v = IVec2::new(3, 4).add(IVec2::new(1, 1));
/// let x = v.x; // 4
/// ```
#[native]
fn ivec2_add(v: I64Vec2, other: I64Vec2) -> Result<I64Vec2, RtErr> {
    v.checked_add(other).ok_or(RtErr::IntegerOverflow)
}

/// Returns this vector minus `other`, component by component. `b.sub(a)` is the vector that
/// points from `a` to `b`.
///
/// ```mimas
/// use std::math::IVec2;
///
/// let player = IVec2::new(1, 1);
/// let enemy = IVec2::new(4, 5);
/// let offset = enemy.sub(player);
/// let x = offset.x; // 3
/// ```
#[native]
fn ivec2_sub(v: I64Vec2, other: I64Vec2) -> Result<I64Vec2, RtErr> {
    v.checked_sub(other).ok_or(RtErr::IntegerOverflow)
}

/// Returns the vector with both components multiplied by `by`.
///
/// ```mimas
/// use std::math::IVec2;
///
/// let cell = IVec2::new(3, 4).scale(8);
/// let x = cell.x; // 24
/// ```
#[native]
fn ivec2_scale(v: I64Vec2, by: i64) -> Result<I64Vec2, RtErr> {
    v.checked_mul(I64Vec2::splat(by))
        .ok_or(RtErr::IntegerOverflow)
}

/// Returns the dot product, `x * other.x + y * other.y`. A result of `0` means the two vectors
/// are perpendicular.
///
/// ```mimas
/// use std::math::IVec2;
///
/// let a = IVec2::new(3, 4).dot(IVec2::new(1, 0)); // 3
/// ```
#[native]
fn ivec2_dot(v: I64Vec2, other: I64Vec2) -> Result<i64, RtErr> {
    dot(v, other)
}

/// Returns the vector's length multiplied by itself. An `IVec2` has no `length`, since that's
/// rarely a whole number, and comparing squared lengths tells which of two is longer.
///
/// ```mimas
/// use std::math::IVec2;
///
/// let n = IVec2::new(3, 4).length_squared(); // 25
/// ```
#[native]
fn ivec2_length_squared(v: I64Vec2) -> Result<i64, RtErr> {
    dot(v, v)
}

/// Returns the distance between the two points multiplied by itself.
///
/// ```mimas
/// use std::math::IVec2;
///
/// let player = IVec2::new(0, 0);
/// let enemy = IVec2::new(3, 4);
/// let close = player.distance_squared(enemy) < 6 * 6; // true
/// ```
#[native]
fn ivec2_distance_squared(v: I64Vec2, other: I64Vec2) -> Result<i64, RtErr> {
    let offset = v.checked_sub(other).ok_or(RtErr::IntegerOverflow)?;
    dot(offset, offset)
}

/// Returns the vector as a `Vec2`. `Vec2.to_ivec2` goes the other way.
///
/// ```mimas
/// use std::math::IVec2;
///
/// let half = IVec2::new(3, 4).to_vec2().scale(0.5);
/// let x = half.x; // 1.5
/// ```
#[native]
fn ivec2_to_vec2(v: I64Vec2) -> Vec2 {
    v.as_vec2()
}

/// Creates the position of cell `index` in a grid that is `width` cells wide, counting across
/// each row and then down. It's `ivec2(index % width, index ~/ width)`, and `to_index` goes the
/// other way.
///
/// A `width` below `1` is a runtime error.
///
/// ```mimas
/// use std::math::IVec2;
///
/// let cell = IVec2::from_index(7, 3);
/// let x = cell.x; // 1
/// let y = cell.y; // 2
/// ```
#[native]
fn ivec2_from_index(index: i64, width: i64) -> Result<I64Vec2, RtErr> {
    if width < 1 {
        return Err(RtErr::InvalidArgument("from_index needs width >= 1".into()));
    }
    Ok(I64Vec2::new(index % width, index / width))
}

/// Returns the number of this cell in a grid that is `width` cells wide, counting across each row
/// and then down. It's `y * width + x`, which lets a grid be one flat array, or a position be
/// kept as a single `int`. `IVec2::from_index` goes the other way.
///
/// ```mimas
/// use std::math::ivec2;
///
/// let cells = [0, 0, 0, 0, 0, 0, 0, 0, 0];
/// cells[ivec2(1, 2).to_index(3)] = 5; // cells[7]
/// ```
#[native]
fn ivec2_to_index(v: I64Vec2, width: i64) -> Result<i64, RtErr> {
    let row = v.y.checked_mul(width);
    let index = row.and_then(|row| row.checked_add(v.x));
    index.ok_or(RtErr::IntegerOverflow)
}

/// Creates a vector from its `x`, `y` and `z` components. A struct literal such as
/// `IVec3 { x = 1, y = 2, z = 3 }` does the same.
///
/// ```mimas
/// use std::math::IVec3;
///
/// let v = IVec3::new(1, 2, 3);
/// let z = v.z; // 3
/// ```
#[native]
fn ivec3_new(x: i64, y: i64, z: i64) -> I64Vec3 {
    I64Vec3::new(x, y, z)
}

/// Returns the sum of the two vectors, adding them component by component.
///
/// ```mimas
/// use std::math::IVec3;
///
/// let v = IVec3::new(1, 2, 3).add(IVec3::new(1, 1, 1));
/// let z = v.z; // 4
/// ```
#[native]
fn ivec3_add(v: I64Vec3, other: I64Vec3) -> Result<I64Vec3, RtErr> {
    v.checked_add(other).ok_or(RtErr::IntegerOverflow)
}

/// Returns this vector minus `other`, component by component.
///
/// ```mimas
/// use std::math::IVec3;
///
/// let v = IVec3::new(4, 5, 6).sub(IVec3::new(1, 1, 1));
/// let z = v.z; // 5
/// ```
#[native]
fn ivec3_sub(v: I64Vec3, other: I64Vec3) -> Result<I64Vec3, RtErr> {
    v.checked_sub(other).ok_or(RtErr::IntegerOverflow)
}

/// Returns the vector with every component multiplied by `by`.
///
/// ```mimas
/// use std::math::IVec3;
///
/// let v = IVec3::new(1, 2, 3).scale(2);
/// let z = v.z; // 6
/// ```
#[native]
fn ivec3_scale(v: I64Vec3, by: i64) -> Result<I64Vec3, RtErr> {
    v.checked_mul(I64Vec3::splat(by))
        .ok_or(RtErr::IntegerOverflow)
}

fn dot(a: I64Vec2, b: I64Vec2) -> Result<i64, RtErr> {
    let products = a.checked_mul(b);
    let sum = products.and_then(|p| p.x.checked_add(p.y));
    sum.ok_or(RtErr::IntegerOverflow)
}
