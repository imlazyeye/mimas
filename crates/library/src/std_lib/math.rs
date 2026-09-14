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
        "the ratio of a circle's circumference to its diameter",
    );
    m.constant(
        "TAU",
        Ty::Float,
        Literal::Float(std::f64::consts::TAU),
        "a full turn in radians, twice PI",
    );
    m.constant(
        "E",
        Ty::Float,
        Literal::Float(std::f64::consts::E),
        "Euler's number",
    );
    m.add_adt::<Vec2>();
    m.add_adt::<Vec3>();
    m.add_adt::<Quat>();
    api.add_assoc_of::<Vec2, _, _>("new", vec2_new);
    api.add_assoc_of::<Vec2, _, _>("from_angle", vec2_from_angle);
    api.add_method_named("length", vec2_length);
    api.add_method_named("normalize", vec2_normalize);
    api.add_method_named("add", vec2_add);
    api.add_method_named("sub", vec2_sub);
    api.add_method_named("scale", vec2_scale);
    api.add_method_named("dot", vec2_dot);
    api.add_method_named("distance", vec2_distance);
    api.add_method_named("angle", vec2_angle);
    api.add_method_named("lerp", vec2_lerp);
    api.add_method_named("length", vec3_length);
    api.add_method_named("normalize", vec3_normalize);
    api.add_assoc_of::<Quat, _, _>("from_rotation_z", quat_from_rotation_z);
    api.add_method_named("rotate_z", quat_rotate_z);
}

#[native]
fn vec2_new(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

#[native]
fn vec2_from_angle(radians: f32) -> Vec2 {
    Vec2::from_angle(radians)
}

#[native]
fn vec2_length(v: Vec2) -> f32 {
    v.length()
}

#[native]
fn vec2_normalize(v: Vec2) -> Vec2 {
    v.normalize_or_zero()
}

#[native]
fn vec2_add(v: Vec2, other: Vec2) -> Vec2 {
    v + other
}

#[native]
fn vec2_sub(v: Vec2, other: Vec2) -> Vec2 {
    v - other
}

#[native]
fn vec2_scale(v: Vec2, by: f32) -> Vec2 {
    v * by
}

#[native]
fn vec2_dot(v: Vec2, other: Vec2) -> f32 {
    v.dot(other)
}

#[native]
fn vec2_distance(v: Vec2, other: Vec2) -> f32 {
    v.distance(other)
}

#[native]
fn vec2_angle(v: Vec2) -> f32 {
    v.to_angle()
}

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

#[native]
fn quat_from_rotation_z(radians: f32) -> Quat {
    Quat::from_rotation_z(radians)
}

#[native]
fn quat_rotate_z(q: Quat, radians: f32) -> Quat {
    q * Quat::from_rotation_z(radians)
}
