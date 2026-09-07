#[macro_use]
mod test_runner;

// len: char count, not byte count
test_run!(
    len_basic,
    r#""hello".len()"# => "5",
    r#""a".len()"# => "1",
    r#""".len()"# => "0",
);

// multibyte: len counts chars, not utf-8 bytes
test_run!(
    len_unicode,
    r#""café".len()"# => "4",
    r#""naïve".len()"# => "5",
    r#""ÄÖÜ".len()"# => "3",
    r#""👍abc".len()"# => "4",
    r#""👍".len()"# => "1",
);

// contains: substring test
test_run!(
    contains_hit,
    r#""hello world".contains("world")"# => "true",
    r#""hello world".contains("hello")"# => "true",
    r#""hello world".contains("o w")"# => "true",
);

test_run!(
    contains_miss,
    r#""hello world".contains("xyz")"# => "false",
    r#""hello".contains("Hello")"# => "false",
);

// empty needle is always contained
test_run!(
    contains_empty_needle,
    r#""hello".contains("")"# => "true",
    r#""".contains("")"# => "true",
);

test_run!(
    contains_empty_haystack,
    r#""".contains("x")"# => "false",
);

test_run!(
    contains_unicode,
    r#""café au lait".contains("é")"# => "true",
    r#""café".contains("naïve")"# => "false",
);

test_run!(
    starts_with_hit,
    r#""hello".starts_with("he")"# => "true",
    r#""hello".starts_with("hello")"# => "true",
);

test_run!(
    starts_with_miss,
    r#""hello".starts_with("lo")"# => "false",
    r#""hello".starts_with("Hello")"# => "false",
);

test_run!(
    starts_with_empty_prefix,
    r#""hello".starts_with("")"# => "true",
    r#""".starts_with("")"# => "true",
);

test_run!(
    ends_with_hit,
    r#""hello".ends_with("lo")"# => "true",
    r#""hello".ends_with("hello")"# => "true",
);

test_run!(
    ends_with_miss,
    r#""hello".ends_with("he")"# => "false",
    r#""hello".ends_with("LO")"# => "false",
);

test_run!(
    ends_with_empty_suffix,
    r#""hello".ends_with("")"# => "true",
    r#""".ends_with("")"# => "true",
);

test_run!(
    to_upper,
    r#""Hello".to_upper()"# => r#""HELLO""#,
    r#""hello".to_upper()"# => r#""HELLO""#,
    r#""HELLO".to_upper()"# => r#""HELLO""#,
    r#""".to_upper()"# => r#""""#,
    r#""café".to_upper()"# => r#""CAFÉ""#,
);

test_run!(
    to_lower,
    r#""Hello".to_lower()"# => r#""hello""#,
    r#""HELLO".to_lower()"# => r#""hello""#,
    r#""hello".to_lower()"# => r#""hello""#,
    r#""".to_lower()"# => r#""""#,
    r#""ÄÖÜ".to_lower()"# => r#""äöü""#,
);

// digits/symbols unaffected by case ops
test_run!(
    case_digits_unchanged,
    r#""abc123!".to_upper()"# => r#""ABC123!""#,
    r#""ABC123!".to_lower()"# => r#""abc123!""#,
);

// trim: leading + trailing whitespace, internal preserved
test_run!(
    trim,
    r#""  hi  ".trim()"# => r#""hi""#,
    "\"\\t hi \\n\".trim()" => r#""hi""#,
    r#""".trim()"# => r#""""#,
    r#""   ".trim()"# => r#""""#,
    r#""hi".trim()"# => r#""hi""#,
    r#""  a b  ".trim()"# => r#""a b""#,
);

// to_snake (heck::ToSnakeCase)
test_run!(
    to_snake,
    r#""hello world foo".to_snake()"# => r#""hello_world_foo""#,
    r#""HelloWorld".to_snake()"# => r#""hello_world""#,
    r#""XMLParser".to_snake()"# => r#""xml_parser""#,
    r#""already_snake".to_snake()"# => r#""already_snake""#,
    r#""HELLO".to_snake()"# => r#""hello""#,
    r#""naïve café".to_snake()"# => r#""naïve_café""#,
);

// split on a delimiter
test_run!(
    split_basic,
    r#""a,b,c".split(",")"# => r#"["a", "b", "c"]"#,
    r#""abc".split(",")"# => r#"["abc"]"#,
    r#""a::b::c".split("::")"# => r#"["a", "b", "c"]"#,
);

// trailing/leading delimiter yields empty segments
test_run!(
    split_edge_empties,
    r#""a,,b".split(",")"# => r#"["a", "", "b"]"#,
    r#"",a".split(",")"# => r#"["", "a"]"#,
);

// split on "" splits into chars, with leading+trailing empty segments
test_run!(
    split_empty_delim,
    r#""abc".split("")"# => r#"["", "a", "b", "c", ""]"#,
    r#""".split("")"# => r#"["", ""]"#,
    r#""".split(",")"# => r#"[""]"#,
);

test_run!(
    split_len,
    r#""a,b,c,d".split(",").len()"# => "4",
    r#""a-b-c-d-e".split("-").len()"# => "5",
);

// find: first regex match, capture groups included; returns [str]? (null on no match)
test_run!(
    find_no_match,
    r#""abc".find("[0-9]+")"# => "null",
    r#""".find("x")"# => "null",
);

test_run!(
    find_match,
    r#""hello123world".find("[0-9]+")"# => r#"["123"]"#,
    r#""hello world".find("world")"# => r#"["world"]"#,
);

test_run!(
    find_capture_groups,
    r#""12-34".find("(\\d+)-(\\d+)")"# => r#"["12-34", "12", "34"]"#,
);

// invalid regex returns null (the .ok()? path), not a fault
test_run!(
    find_invalid_regex_null,
    r#""abc".find("(")"# => "null",
);

// find_all: every match; each element is the captures of one match. [[str]]? null on bad regex
test_run!(
    find_all_multiple,
    r#""a1b2c3".find_all("[0-9]")"# => r#"[["1"], ["2"], ["3"]]"#,
);

test_run!(
    find_all_no_match,
    r#""xyz".find_all("[0-9]")"# => "[]",
);

test_run!(
    find_all_capture_groups,
    r#""a1 b2".find_all("([a-z])([0-9])")"# => r#"[["a1", "a", "1"], ["b2", "b", "2"]]"#,
);

test_run!(
    find_all_invalid_regex_null,
    r#""abc".find_all("(")"# => "null",
);

// lines: split on newlines
test_run!(
    lines,
    r#""a\nb\nc".lines()"# => r#"["a", "b", "c"]"#,
    r#""".lines()"# => "[]",
    r#""justone".lines()"# => r#"["justone"]"#,
    r#""a\nb\n".lines()"# => r#"["a", "b"]"#,
    r#""a\nb\nc\nd".lines().len()"# => "4",
);

// capitalize: uppercase the first char, rest verbatim
test_run!(
    capitalize,
    r#""hello".capitalize()"# => r#""Hello""#,
    r#""".capitalize()"# => r#""""#,
    r#""Hello".capitalize()"# => r#""Hello""#,
    r#""hELLO".capitalize()"# => r#""HELLO""#,
    r#""über".capitalize()"# => r#""Über""#,
    r#""x".capitalize()"# => r#""X""#,
);

// to_int: trims then parses i64, returns int? (null on failure)
test_run!(
    to_int_basic,
    r#""42".to_int()"# => "42",
    r#""0".to_int()"# => "0",
    r#""-17".to_int()"# => "-17",
    r#""+5".to_int()"# => "5",
    r#""  42  ".to_int()"# => "42",
    r#""  -3  ".to_int()"# => "-3",
);

// non-numeric / empty / float / hex -> null
test_run!(
    to_int_null,
    r#""notanum".to_int()"# => "null",
    r#""".to_int()"# => "null",
    r#""3.5".to_int()"# => "null",
    r#""0x10".to_int()"# => "null",
    r#""99999999999999999999".to_int()"# => "null",
);

test_run!(
    to_int_i64_max,
    r#""9223372036854775807".to_int()"# => "9223372036854775807",
);

// usable as a real int when present
test_run!(
    to_int_typed,
    r#"let n: int = "10".to_int() ?? 0;"#,
    "n" => "10",
);

// ord: first char's unicode codepoint, int? (null on empty)
test_run!(
    ord,
    r#""A".ord()"# => "65",
    r#""a".ord()"# => "97",
    r#""0".ord()"# => "48",
    r#""ABC".ord()"# => "65",
    r#""é".ord()"# => "233",
    r#""👍".ord()"# => "128077",
    r#""".ord()"# => "null",
);

test_run!(
    is_empty,
    r#""".is_empty()"# => "true",
    r#""x".is_empty()"# => "false",
    r#"" ".is_empty()"# => "false",
);

// replace: every occurrence
test_run!(
    replace,
    r#""aaa".replace("a", "b")"# => r#""bbb""#,
    r#""hello".replace("xyz", "q")"# => r#""hello""#,
    r#""foo bar foo".replace("foo", "baz")"# => r#""baz bar baz""#,
    r#""hello".replace("ll", "")"# => r#""heo""#,
    r#""café".replace("é", "e")"# => r#""cafe""#,
);

// empty needle inserts between every char (Rust's str::replace semantics)
test_run!(
    replace_empty_needle,
    r#""aaa".replace("", "x")"# => r#""xaxaxax""#,
);

test_run!(
    repeat,
    r#""ab".repeat(3)"# => r#""ababab""#,
    r#""ab".repeat(1)"# => r#""ab""#,
    r#""foo".repeat(0)"# => r#""""#,
    r#""".repeat(5)"# => r#""""#,
    r#""café".repeat(2).len()"# => "8",
    r#""ab".repeat(4).len()"# => "8",
);

test_fail!(repeat_rejects_negative, r#"let _ = "foo".repeat(-1);"#);
test_fail!(
    repeat_rejects_overflowing_alloc,
    r#"let _ = "foo".repeat(9000000000000000000);"#
);

test_run!(
    chains,
    r#""  Hello  ".trim().to_upper()"# => r#""HELLO""#,
    r#""HELLO".to_lower().len()"# => "5",
    r#""a_b_c".replace("_", "-").contains("-")"# => "true",
);

// option-chaining through a nullable str
test_run!(
    chain_null_str,
    "let s: str? = null;",
    "s?.len()" => "null",
    "s?.to_upper()" => "null",
);

test_run!(
    chain_alive_str,
    r#"let s: str? = "hi";"#,
    "s?.len()" => "2",
    "s?.to_upper() ?? \"x\"" => r#""HI""#,
);

test_fail!(str_has_no_pop, r#""hi".pop();"#);
test_fail!(str_has_no_flatten, r#""hi".flatten();"#);
