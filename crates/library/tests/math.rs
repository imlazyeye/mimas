#[macro_use]
mod test_runner;

test_run!(
    vec2_math,
    "use std::math::Vec2;",
    "Vec2::new(3.0, 4.0).length()" => "5",
    "Vec2 { x = 1.0, y = 2.0 }.add(Vec2::new(2.0, 3.0)).y" => "5",
    "Vec2::new(0.0, 2.0).normalize().y" => "1",
    "Vec2::from_angle(0.0).x" => "1",
    "Vec2::zero().x" => "0",
    "Vec2::new(3.0, 4.0).length_squared()" => "25",
    "Vec2::new(1.0, 1.0).distance_squared(Vec2::new(4.0, 5.0))" => "25",
    "Vec2::new(1.0, 0.0).rotate(std::math::PI / 2.0).y" => "1",
    "Vec2::new(1.0, 0.0).perp().y" => "1",
    "Vec2::new(0.0, 1.0).perp().x" => "-1",
    "Vec2::new(3.7, -1.2).to_ints()" => "[3, -1]",
);

test_run!(
    vec3_and_quat_math,
    "use std::math::{Vec3, Quat};",
    "Vec3 { x = 2.0, y = 3.0, z = 6.0 }.length()" => "7",
    "Quat::from_rotation_z(0.0).rotate_z(0.0).w" => "1",
);
