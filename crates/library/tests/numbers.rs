#[macro_use]
mod test_runner;

test_run!(
    int_abs,
    "(7).abs()" => "7",
    "(0).abs()" => "0",
    "(-5).abs()" => "5",
    "(-1).abs()" => "1",
);

test_run!(
    int_min,
    "(2).min(7)" => "2",
    "(7).min(2)" => "2",
    "(10).min(-3)" => "-3",
    "(4).min(4)" => "4",
);

test_run!(
    int_max,
    "(2).max(7)" => "7",
    "(7).max(2)" => "7",
    "(10).max(-3)" => "10",
    "(4).max(4)" => "4",
);

test_run!(
    int_clamp_in_range,
    "(5).clamp(1, 10)" => "5",
    "(1).clamp(1, 10)" => "1",
    "(10).clamp(1, 10)" => "10",
);

test_run!(
    int_clamp_below,
    "(-3).clamp(1, 10)" => "1",
    "(0).clamp(1, 10)" => "1",
);

test_run!(
    int_clamp_above,
    "(20).clamp(1, 10)" => "10",
    "(11).clamp(1, 10)" => "10",
);

test_run!(
    int_clamp_negative_range,
    "(-5).clamp(-10, -1)" => "-5",
    "(-50).clamp(-10, -1)" => "-10",
    "(50).clamp(-10, -1)" => "-1",
);

test_run!(
    int_to_str,
    "(42).to_str()" => r#""42""#,
    "(0).to_str()" => r#""0""#,
    "(-7).to_str()" => r#""-7""#,
);

test_run!(
    int_to_float,
    "(3).to_float()" => "3",
    "(0).to_float()" => "0",
    "(-2).to_float()" => "-2",
);

// to_float yields a real float (chains into float-only methods)
test_run!(
    int_to_float_is_float,
    "(2).to_float().sqrt()" => "1.4142135623730951",
    "(9).to_float().sqrt()" => "3",
);

test_run!(
    int_methods_chain,
    "(-5).abs().min(3)" => "3",
    "(-20).clamp(1, 10).max(5)" => "5",
);

// int::random(n) is uniform in 0..n; assert the range, not the value
test_run!(
    int_random,
    "int::random(1)" => "0",
);

test_run!(
    int_random_in_range,
    "let n = int::random(100);",
    "n >= 0 && n < 100" => "true",
);

test_run!(
    int_random_small_range,
    "let n = int::random(2);",
    "n == 0 || n == 1" => "true",
);

test_run!(
    float_floor,
    "(3.7).floor()" => "3",
    "(3.0).floor()" => "3",
    "(-3.2).floor()" => "-4",
    "(-3.0).floor()" => "-3",
);

test_run!(
    float_ceil,
    "(3.2).ceil()" => "4",
    "(3.0).ceil()" => "3",
    "(-3.7).ceil()" => "-3",
    "(-3.0).ceil()" => "-3",
);

test_run!(
    float_round,
    "(3.2).round()" => "3",
    "(3.5).round()" => "4",
    "(2.5).round()" => "3",
    "(-3.5).round()" => "-4",
    "(-2.5).round()" => "-3",
);

// to_int truncates toward zero (as i64), not floor
test_run!(
    float_to_int_positive,
    "(3.7).to_int()" => "3",
    "(3.2).to_int()" => "3",
    "(3.0).to_int()" => "3",
);

test_run!(
    float_to_int_negative_truncates_toward_zero,
    "(-3.7).to_int()" => "-3",
    "(-3.2).to_int()" => "-3",
    "(-0.9).to_int()" => "0",
);

// to_int truncates, floor rounds down -- they differ for negatives
test_run!(
    float_to_int_differs_from_floor,
    "(-3.5).to_int()" => "-3",
    "(-3.5).floor()" => "-4",
);

test_run!(
    float_abs,
    "(4.5).abs()" => "4.5",
    "(0.0).abs()" => "0",
    "(-4.0).abs()" => "4",
    "(-2.5).abs()" => "2.5",
);

test_run!(
    float_max,
    "(2.5).max(7.5)" => "7.5",
    "(7.5).max(2.5)" => "7.5",
    "(3.0).max(-1.0)" => "3",
);

test_run!(
    float_min,
    "(2.5).min(7.5)" => "2.5",
    "(7.5).min(2.5)" => "2.5",
    "(-1.0).min(3.0)" => "-1",
);

test_run!(
    float_sqrt,
    "(9.0).sqrt()" => "3",
    "(4.0).sqrt()" => "2",
    "(2.0).sqrt()" => "1.4142135623730951",
    "(0.0).sqrt()" => "0",
);

test_run!(
    float_format,
    r#"(3.14159).format(0)"# => r#""3""#,
    r#"(3.14159).format(2)"# => r#""3.14""#,
    r#"(3.14159).format(4)"# => r#""3.1416""#,
    r#"(1.0).format(3)"# => r#""1.000""#,
);

// format rounds at the requested precision (half-to-even on the binary repr)
test_run!(
    float_format_rounding,
    r#"(2.5).format(0)"# => r#""2""#,
    r#"(0.125).format(2)"# => r#""0.12""#,
    r#"(3.005).format(2)"# => r#""3.00""#,
);

test_run!(
    float_format_negative,
    r#"(-3.14159).format(2)"# => r#""-3.14""#,
);

test_run!(
    float_methods_chain,
    "(-2.5).abs().floor()" => "2",
    "(7.3).floor().to_int()" => "7",
);

// float::random(n) is uniform in 0.0..n; assert the range, not the value
test_run!(
    float_random_in_range,
    "let n = float::random(1.0);",
    "n >= 0.0 && n < 1.0" => "true",
);

test_run!(
    float_random_larger_range,
    "let n = float::random(10.0);",
    "n >= 0.0 && n < 10.0" => "true",
);

// bool::random() returns a bool; we can't pin the value, but it equals itself
test_run!(
    bool_random_is_bool,
    "let b = bool::random();",
    "b == b" => "true",
    "b || !b" => "true",
);

test_run!(
    float_to_str,
    r#"(3.5).to_str()"# => r#""3.5""#,
    r#"(3.0).to_str()"# => r#""3""#,
);

// int has no float-only methods
test_fail!(
    int_no_float_methods,
    "let x = (9).sqrt();",
    "let x = (3).floor();",
    "let x = (3).format(2);"
);

// float has no int-only clamp
test_fail!(float_no_clamp, "let x = (3.5).clamp(1.0, 10.0);");

// arity / type mismatches
test_fail!(
    method_arity_and_type_mismatches,
    "let x = (3).min();",
    "let x = (3).clamp(1);",
    "let x = (3.5).max(2);",
    "let x = (3).min(2.0);",
    "let x = (3.5).format(2.0);"
);

// instance method dispatch on a user struct
test_run!(
    instance_method_dispatch,
    "struct Pair { a: int, b: int }
     impl Pair {
         fn sum(self) -> int { self.a + self.b }
         fn scaled(self, k: int) -> int { self.sum() * k }
     }
     let p = Pair { a = 3, b = 4 };",
    "p.sum()" => "7",
    "p.scaled(2)" => "14",
);

// std::math::PI
test_run!(
    math_pi_value,
    "use std::math::PI;",
    "PI" => "3.141592653589793",
);

test_run!(
    math_pi_is_float,
    "use std::math::PI;
     let p: float = PI;",
    "p" => "3.141592653589793",
);

test_run!(
    math_pi_arithmetic,
    "use std::math::PI;",
    "PI * 2.0" => "6.283185307179586",
    "PI > 3.0" => "true",
    "PI < 4.0" => "true",
);

test_run!(
    math_pi_in_expression,
    "use std::math::PI;
     let r: float = 2.0;",
    "PI * r * r" => "12.566370614359172",
);

// int literal coerces to float in float arithmetic, so PI + 1 is valid float
test_run!(
    math_pi_int_literal_coerces,
    "use std::math::PI;",
    "PI + 1" => "4.141592653589793",
);

// PI is float, not int -- binding it to an int slot must be rejected
test_fail!(math_pi_not_int, "use std::math::PI; let _: int = PI;");
