use macros::native;
use shared::{Literal, Ty};
use vm::{
    api::Api,
    glam::{Quat, Vec2, Vec3},
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
    api.add_method_named("to_ints", vec2_to_ints);
    api.add_method_named("angle", vec2_angle);
    api.add_method_named("lerp", vec2_lerp);
    api.add_method_named("length", vec3_length);
    api.add_method_named("normalize", vec3_normalize);
    api.add_assoc_of::<Quat, _, _>("from_rotation_z", quat_from_rotation_z);
    api.add_method_named("rotate_z", quat_rotate_z);
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

/// Returns the components as a pair of `int`s, each truncated toward zero the way `float.to_int`
/// does. It's the step from a position to the pixel it's in.
///
/// ```mimas
/// use std::math::Vec2;
///
/// let (x, y) = Vec2::new(3.7, -1.2).to_ints(); // (3, -1)
/// ```
#[native]
fn vec2_to_ints(v: Vec2) -> (i64, i64) {
    (v.x as i64, v.y as i64)
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
