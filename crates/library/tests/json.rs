#[macro_use]
mod test_runner;

test_run!(
    from_json_to_json_object_roundtrip,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("{\"name\": \"bob\"}")!)!"# => r#""{\"name\":\"bob\"}""#,
);

test_run!(
    from_json_to_json_array_roundtrip,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("[1, 2, 3]")!)!"# => r#""[1,2,3]""#,
);

test_run!(
    from_json_to_json_nested_roundtrip,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("{\"a\": {\"b\": {\"c\": [1, 2]}}}")!)!"# => r#""{\"a\":{\"b\":{\"c\":[1,2]}}}""#,
);

test_run!(
    from_json_to_json_empty_object,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("{}")!)!"# => r#""{}""#,
);

test_run!(
    from_json_to_json_empty_array,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("[]")!)!"# => r#""[]""#,
);

test_run!(
    from_json_to_json_string_roundtrip,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("\"hello\"")!)!"# => r#""\"hello\"""#,
);

test_run!(
    from_json_to_json_bool_true,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("true")!)!"# => r#""true""#,
);

test_run!(
    from_json_to_json_bool_false,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("false")!)!"# => r#""false""#,
);

test_run!(
    from_json_to_json_null,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("null")!)!"# => r#""null""#,
);

test_run!(
    from_json_to_json_int_number,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("42")!)!"# => r#""42""#,
);

test_run!(
    from_json_to_json_negative_int,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("-7")!)!"# => r#""-7""#,
);

test_run!(
    from_json_to_json_float_number,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("3.5")!)!"# => r#""3.5""#,
);

// whole-number floats round-trip back out as ints (parse.rs:95 special case)
test_run!(
    whole_float_roundtrips_as_int,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("1.0")!)!"# => r#""1""#,
    r#"parse::to_json(parse::from_json("[1.0, 2.0, 3.0]")!)!"# => r#""[1,2,3]""#,
);

test_run!(
    mixed_int_and_float_array_roundtrip,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("[1, 2.5, 3]")!)!"# => r#""[1,2.5,3]""#,
);

test_run!(
    bool_and_null_array_roundtrip,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("[true, false, null]")!)!"# => r#""[true,false,null]""#,
);

test_run!(
    big_whole_number_roundtrip,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("1000000")!)!"# => r#""1000000""#,
);

test_run!(
    as_str_on_string,
    "use std::parse;
     let v = parse::from_json(\"\\\"world\\\"\")!;",
    "v.as_str()!" => r#""world""#,
);

test_run!(
    as_str_on_object_key_value,
    "use std::parse;
     let obj = parse::from_json(\"{\\\"name\\\": \\\"alice\\\"}\")!;
     let d = obj.as_dict()!;
     let nameval = d[\"name\"]!;",
    "nameval.as_str()!" => r#""alice""#,
);

// wrong variant -> null (option None)
test_run!(
    as_str_wrong_variant_is_null,
    "use std::parse;
     let n = parse::from_json(\"42\")!;
     let b = parse::from_json(\"true\")!;
     let z = parse::from_json(\"null\")!;
     let a = parse::from_json(\"[1]\")!;
     let o = parse::from_json(\"{}\")!;",
    "n.as_str()" => "null",
    "b.as_str()" => "null",
    "z.as_str()" => "null",
    "a.as_str()" => "null",
    "o.as_str()" => "null",
);

test_run!(
    as_float_on_int_number,
    "use std::parse;
     let v = parse::from_json(\"9\")!;",
    "v.as_float()!" => "9",
);

test_run!(
    as_float_on_float_number,
    "use std::parse;
     let v = parse::from_json(\"3.14\")!;",
    "v.as_float()!" => "3.14",
);

test_run!(
    as_float_on_negative,
    "use std::parse;
     let v = parse::from_json(\"-3.5\")!;",
    "v.as_float()!" => "-3.5",
);

test_run!(
    as_float_on_zero,
    "use std::parse;
     let v = parse::from_json(\"0\")!;",
    "v.as_float()!" => "0",
);

// wrong variant -> null
test_run!(
    as_float_wrong_variant_is_null,
    "use std::parse;
     let s = parse::from_json(\"\\\"x\\\"\")!;
     let b = parse::from_json(\"true\")!;
     let z = parse::from_json(\"null\")!;",
    "s.as_float()" => "null",
    "b.as_float()" => "null",
    "z.as_float()" => "null",
);

test_run!(
    as_array_len,
    "use std::parse;
     let v = parse::from_json(\"[10, 20, 30]\")!;
     let a = v.as_array()!;",
    "a.len()" => "3",
);

test_run!(
    as_array_element_as_float,
    "use std::parse;
     let v = parse::from_json(\"[10, 20, 30]\")!;
     let a = v.as_array()!;
     let first = a[0];",
    "first.as_float()!" => "10",
);

test_run!(
    as_array_empty_len,
    "use std::parse;
     let v = parse::from_json(\"[]\")!;
     let a = v.as_array()!;",
    "a.len()" => "0",
);

// wrong variant -> null
test_run!(
    as_array_wrong_variant_is_null,
    "use std::parse;
     let o = parse::from_json(\"{}\")!;
     let s = parse::from_json(\"\\\"x\\\"\")!;",
    "o.as_array()" => "null",
    "s.as_array()" => "null",
);

test_run!(
    as_dict_len,
    "use std::parse;
     let v = parse::from_json(\"{\\\"a\\\": 1, \\\"b\\\": 2}\")!;
     let d = v.as_dict()!;",
    "d.len()" => "2",
);

test_run!(
    as_dict_contains_key,
    "use std::parse;
     let v = parse::from_json(\"{\\\"name\\\": \\\"x\\\"}\")!;
     let d = v.as_dict()!;",
    "d.contains_key(\"name\")" => "true",
    "d.contains_key(\"missing\")" => "false",
);

test_run!(
    as_dict_value_lookup,
    "use std::parse;
     let v = parse::from_json(\"{\\\"n\\\": 7}\")!;
     let d = v.as_dict()!;
     let nv = d[\"n\"]!;",
    "nv.as_float()!" => "7",
);

test_run!(
    as_dict_empty_len,
    "use std::parse;
     let v = parse::from_json(\"{}\")!;
     let d = v.as_dict()!;",
    "d.len()" => "0",
);

// wrong variant -> null
test_run!(
    as_dict_wrong_variant_is_null,
    "use std::parse;
     let a = parse::from_json(\"[1]\")!;
     let n = parse::from_json(\"5\")!;",
    "a.as_dict()" => "null",
    "n.as_dict()" => "null",
);

// deep extraction through nested structure
test_run!(
    nested_object_array_extraction,
    "use std::parse;
     let root = parse::from_json(\"{\\\"users\\\": [{\\\"age\\\": 30}]}\")!;
     let d = root.as_dict()!;
     let usersval = d[\"users\"]!;
     let users = usersval.as_array()!;
     let user0 = users[0];
     let u = user0.as_dict()!;
     let ageval = u[\"age\"]!;",
    "ageval.as_float()!" => "30",
);

// malformed json -> from_json raises; `!` on a raise faults the VM
test_fail!(
    invalid_json_faults,
    r#"use std::parse; let x = parse::from_json("{")!;"#,
    r#"use std::parse; let x = parse::from_json("{not valid}")!;"#,
    r#"use std::parse; let x = parse::from_json("[1, 2,]")!;"#,
    r#"use std::parse; let x = parse::from_json("hello")!;"#,
    r#"use std::parse; let x = parse::from_json("")!;"#,
    r#"use std::parse; let x = parse::from_json("{'a': 1}")!;"#,
);

// invalid json recovered with absolve keeps running and yields the fallback
test_run!(
    invalid_json_absolve_fallback,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("{bad}") absolve |_| parse::from_json("null")!)!"# => r#""null""#,
);

test_run!(
    valid_json_absolve_passes_through,
    "use std::parse;",
    r#"parse::to_json(parse::from_json("[1]") absolve |_| parse::from_json("null")!)!"# => r#""[1]""#,
);
