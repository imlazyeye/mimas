#[macro_use]
mod test_runner;

test_run!(
    vec2_math,
    "use std::math::Vec2;",
    "Vec2::new(3.0, 4.0).length()" => "5",
    "Vec2 { x = 1.0, y = 2.0 }.add(Vec2::new(2.0, 3.0)).y" => "5",
    "Vec2::new(0.0, 2.0).normalize().y" => "1",
    "Vec2::from_angle(0.0).x" => "1",
);

test_run!(
    vec3_and_quat_math,
    "use std::math::{Vec3, Quat};",
    "Vec3 { x = 2.0, y = 3.0, z = 6.0 }.length()" => "7",
    "Quat::from_rotation_z(0.0).rotate_z(0.0).w" => "1",
);
