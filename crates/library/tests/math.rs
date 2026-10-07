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
    "Vec2::new(3.7, -1.2).to_ivec2().x" => "3",
    "Vec2::new(3.7, -1.2).to_ivec2().y" => "-1",
);

test_run!(
    vec3_and_quat_math,
    "use std::math::{Vec3, Quat};",
    "Vec3 { x = 2.0, y = 3.0, z = 6.0 }.length()" => "7",
    "Quat::from_rotation_z(0.0).rotate_z(0.0).w" => "1",
);

test_run!(
    ivec2_math,
    "use std::math::IVec2;",
    "IVec2 { x = 1, y = 2 }.add(IVec2::new(2, 3)).y" => "5",
    "IVec2::new(4, 5).sub(IVec2::new(1, 1)).x" => "3",
    "IVec2::zero().x" => "0",
    "IVec2::new(3, 4).scale(8).y" => "32",
    "IVec2::new(3, 4).dot(IVec2::new(1, 0))" => "3",
    "IVec2::new(3, 4).length_squared()" => "25",
    "IVec2::new(1, 1).distance_squared(IVec2::new(4, 5))" => "25",
    "IVec2::new(3, 4).to_vec2().scale(0.5).x" => "1.5",
    "IVec2::new(1, 2) == IVec2::new(1, 2)" => "true",
    "IVec2::new(1, 2).to_index(3)" => "7",
    "IVec2::from_index(7, 3) == IVec2::new(1, 2)" => "true",
    "IVec2::from_index(IVec2::new(31, 17).to_index(32), 32).y" => "17",
);

test_run!(
    ivec3_math,
    "use std::math::IVec3;",
    "IVec3 { x = 1, y = 2, z = 3 }.add(IVec3::new(1, 1, 1)).z" => "4",
    "IVec3::new(4, 5, 6).sub(IVec3::new(1, 1, 1)).x" => "3",
    "IVec3::new(1, 2, 3).scale(2).z" => "6",
);

test_fail!(
    ivec2_overflow,
    "use std::math::IVec2;
     let big = IVec2::new(9223372036854775807, 0);
     print(big.add(IVec2::new(1, 0)).x);",
    "use std::math::IVec2;
     let big = IVec2::new(9223372036854775807, 0);
     print(big.scale(2).x);",
    "use std::math::IVec2;
     let big = IVec2::new(9223372036854775807, 0);
     print(big.length_squared());",
    "use std::math::IVec2;
     let big = IVec2::new(1, 9223372036854775807);
     print(big.to_index(2));",
    "use std::math::IVec2;
     print(IVec2::from_index(5, 0).x);",
);

test_run!(
    short_constructors,
    "use std::math::{ivec2, ivec3, vec2, vec3};",
    "vec2(3.0, 4.0).length()" => "5",
    "vec3(2.0, 3.0, 6.0).length()" => "7",
    "ivec2(3, 4) == std::math::IVec2::new(3, 4)" => "true",
    "ivec3(1, 2, 3).z" => "3",
);
