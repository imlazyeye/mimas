#[macro_use]
mod test_runner;

// integer overflow
test_fail!(
    overflow_faults,
    "let x = 9223372036854775807 + 1; print(x);",
    "let big = 9223372036854775807;
     let x = big + 1;
     print(x);",
    "let a = -9223372036854775807;
     let b = a - 2;
     print(b);",
    "let a = 9223372036854775807;
     let b = a * 2;
     print(b);",
    "let a = -9223372036854775807;
     let b = (a - 1) * -1;
     print(b);"
);

// divide / modulo by zero (the integer floor-div operator is ~/)
test_fail!(
    div_mod_by_zero_faults,
    "print(5 ~/ 0);",
    "let d = 0; print(5 ~/ d);",
    "print(5 % 0);",
    "let d = 0; print(7 % d);"
);

// array index out of bounds
test_fail!(
    index_out_of_bounds_faults,
    "let a = [0]; print(a[10]);",
    "let a = [0]; print(a[-1]);",
    "let a = [1, 2, 3]; let i = -5; print(a[i]);",
    "let a: [int] = []; print(a[0]);",
    "let a = [1, 2, 3]; print(a[3]);"
);

// unwrapping null / a missing dict key with !
test_fail!(
    unwrap_null_faults,
    "let x: int? = null; print(x!);",
    r#"let d = ~{ a = 1 }; print(d["missing"]!);"#
);

// unwrapping a raised result with !
test_fail!(
    unwrap_raised_result,
    "fn f() -> int! { raise \"e\" }
     print(f()!);"
);

// match panic terminator (trailing `!`) reached
test_fail!(
    match_panic_terminator_reached,
    r#"let n = 5;
     let x = match n {
       1 => "one",
       2 => "two",
       ! ,
     };
     print(x);"#
);

// invalid bit shift (>= 64, or negative)
test_fail!(
    invalid_shift_faults,
    "print(1 << 64);",
    "let s = 64; print(1 << s);",
    "let s = -1; print(1 << s);",
    "let s = 64; print(256 >> s);"
);

// user panic / todo fault
test_fail!(
    user_panic_todo_faults,
    r#"panic("boom");"#,
    r#"let x = panic("nope"); print(x);"#,
    r#"print(todo("not done yet"));"#
);

// out-of-range int into a narrow/unsigned native param panics the host instead of faulting
test_fail!(
    native_arg_out_of_range_ices,
    "print(\"abc\".repeat(-3));",
    "print((3.14).format(-1));"
);

// native math/string ops fault cleanly
test_fail!(
    native_extreme_arg_ices,
    "print(\"abc\".repeat(9000000000000000000));",
    "print((3.14).format(9000000000000000000));",
    "print((5).clamp(10, 2));"
);
