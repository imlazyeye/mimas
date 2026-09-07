#[macro_use]
mod test_runner;

// real-stdlib option producers feeding a let-else: `str.to_int` and `array.pop` both yield `T?`,
// and `panic` is a native divergent `else`.
test_run!(
    let_else_str_to_int,
    "fn parse_or(s: str, d: int) -> int { let n? = s.to_int() else return d; n }",
    r#"parse_or("42", -1)"# => "42",
    r#"parse_or("abc", -1)"# => "-1",
);

test_run!(
    let_else_array_pop,
    "fn last_or(a: [int], d: int) -> int { let v? = a.pop() else return d; v }",
    "last_or([1, 2, 3], -1)" => "3",
    "last_or([], -1)" => "-1",
);

test_run!(
    let_else_panic_diverges,
    "fn must_parse(s: str) -> int { let n? = s.to_int() else panic(\"not a number\"); n }",
    r#"must_parse("7")"# => "7",
);
