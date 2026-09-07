#[macro_use]
mod test_runner;

test_run!(
    len_basic,
    r#"let d = ~{ a = 1, b = 2, c = 3 };"#,
    "d.len()" => "3",
);

test_run!(
    len_single,
    r#"let d = ~{ only = 1 };"#,
    "d.len()" => "1",
);

test_run!(
    len_empty,
    "let d: ~{int} = ~{};",
    "d.len()" => "0",
);

test_run!(
    len_str_values,
    r#"let d = ~{ a = "x", b = "y" };"#,
    "d.len()" => "2",
);

test_run!(
    len_after_insert,
    r#"let d = ~{ a = 1 };
       d.insert("b", 2);
       d.insert("c", 3);"#,
    "d.len()" => "3",
);

test_run!(
    len_after_remove,
    r#"let d = ~{ a = 1, b = 2, c = 3 };
       d.remove("b");"#,
    "d.len()" => "2",
);

test_run!(
    len_insert_overwrite_no_growth,
    r#"let d = ~{ a = 1 };
       d.insert("a", 99);"#,
    "d.len()" => "1",
);

test_run!(
    len_remove_absent_no_shrink,
    r#"let d = ~{ a = 1, b = 2 };
       d.remove("zzz");"#,
    "d.len()" => "2",
);

test_run!(
    contains_key_present,
    r#"let d = ~{ foo = 1, bar = 2 };"#,
    r#"d.contains_key("foo")"# => "true",
    r#"d.contains_key("bar")"# => "true",
);

test_run!(
    contains_key_absent,
    r#"let d = ~{ foo = 1 };"#,
    r#"d.contains_key("baz")"# => "false",
    r#"d.contains_key("")"# => "false",
);

test_run!(
    contains_key_empty,
    "let d: ~{int} = ~{};",
    r#"d.contains_key("anything")"# => "false",
);

test_run!(
    contains_key_after_insert,
    r#"let d = ~{ a = 1 };
       d.insert("new", 2);"#,
    r#"d.contains_key("new")"# => "true",
);

test_run!(
    contains_key_after_remove,
    r#"let d = ~{ a = 1, b = 2 };
       d.remove("a");"#,
    r#"d.contains_key("a")"# => "false",
    r#"d.contains_key("b")"# => "true",
);

// key must be a str; non-str key is a solve-time mismatch
test_fail!(
    contains_key_rejects_int_key,
    r#"let d = ~{ a = 1 }; let _ = d.contains_key(5);"#
);

// dict index -> V?
test_run!(
    index_present_yields_value,
    r#"let d = ~{ a = 10, b = 20 };"#,
    r#"d["a"]"# => "10",
    r#"d["b"]"# => "20",
);

test_run!(
    index_absent_yields_null,
    r#"let d = ~{ a = 1 };"#,
    r#"d["nope"]"# => "null",
);

test_run!(
    index_empty_yields_null,
    "let d: ~{int} = ~{};",
    r#"d["x"]"# => "null",
);

test_run!(
    index_coalesce,
    r#"let d = ~{ a = 1 };"#,
    r#"d["a"] ?? -1"# => "1",
    r#"d["z"] ?? -1"# => "-1",
);

test_run!(
    index_unwrap_present,
    r#"let d = ~{ a = 7 };"#,
    r#"d["a"]!"# => "7",
);

test_run!(
    index_into_typed_slot,
    r#"let d = ~{ a = 1 };
       let v: int? = d["a"];
       let m: int? = d["z"];"#,
    "v" => "1",
    "m" => "null",
);

// nested option-chaining through dict indexing
test_run!(
    chain_propagation,
    r#"let nested = ~{ a = ~{ b = ~{ c = 9 } } };
       let hole: ~{~{~{int}}} = ~{ a = ~{} };"#,
    r#"nested["a"]?["b"]?["c"]"# => "9",
    r#"hole["a"]?["b"]?["c"]"# => "null",
);

// non-str index key rejected (dicts are string-keyed)
test_fail!(
    index_rejects_int_key,
    r#"let d = ~{ a = 1 }; let _ = d[5];"#
);

// dict.insert -> previous V?
test_run!(
    insert_overwrite_returns_old,
    r#"let d = ~{ a = 1 };
       let prev: int? = d.insert("a", 99);"#,
    "prev" => "1",
    r#"d["a"]"# => "99",
);

test_run!(
    insert_new_key_returns_null,
    r#"let d = ~{ a = 1 };
       let prev: int? = d.insert("b", 2);"#,
    "prev" => "null",
    r#"d["b"]"# => "2",
);

test_run!(
    insert_into_empty,
    r#"let d: ~{int} = ~{};
       let prev: int? = d.insert("first", 5);"#,
    "prev" => "null",
    "d.len()" => "1",
    r#"d["first"]"# => "5",
);

test_run!(
    insert_str_values,
    r#"let d = ~{ a = "x" };
       let prev: str? = d.insert("a", "y");"#,
    "prev" => r#""x""#,
    r#"d["a"]"# => r#""y""#,
);

test_run!(
    insert_repeated_overwrite,
    r#"let d = ~{ k = 0 };
       d.insert("k", 1);
       let prev: int? = d.insert("k", 2);"#,
    "prev" => "1",
    r#"d["k"]"# => "2",
);

// dict.remove -> removed V?
test_run!(
    remove_present_returns_value,
    r#"let d = ~{ a = 1, b = 2 };
       let r: int? = d.remove("b");"#,
    "r" => "2",
    r#"d.contains_key("b")"# => "false",
    "d.len()" => "1",
);

test_run!(
    remove_absent_returns_null,
    r#"let d = ~{ a = 1 };
       let r: int? = d.remove("missing");"#,
    "r" => "null",
    "d.len()" => "1",
);

test_run!(
    remove_from_empty,
    r#"let d: ~{int} = ~{};
       let r: int? = d.remove("x");"#,
    "r" => "null",
);

test_run!(
    remove_then_reinsert,
    r#"let d = ~{ a = 1 };
       d.remove("a");
       let prev: int? = d.insert("a", 5);"#,
    "prev" => "null",
    r#"d["a"]"# => "5",
);

test_run!(
    remove_all_keys,
    r#"let d = ~{ a = 1, b = 2 };
       d.remove("a");
       d.remove("b");"#,
    "d.len()" => "0",
    r#"d.contains_key("a")"# => "false",
);

// dict.pairs print like nested arrays (untyped/variadic render path)
test_run!(
    pairs_single,
    r#"let d = ~{ a = 1 };"#,
    "d.pairs()" => r#"[["a", 1]]"#,
);

test_run!(
    pairs_empty,
    "let d: ~{int} = ~{};",
    "d.pairs()" => "[]",
);

test_run!(
    pairs_iterate_sum_values,
    r#"let d = ~{ a = 10, b = 20 };
       let total = 0;
       for p in d.pairs() { total = total + p.1; }"#,
    "total" => "30",
);

test_run!(
    pairs_len_and_index,
    r#"let d = ~{ a = 1 };
       let p = d.pairs();"#,
    "p.len()" => "1",
    "p[0].1" => "1",
);

test_run!(
    insert_inline_coalesce,
    r#"let d = ~{ a = 1 };"#,
    r#"d.insert("a", 5) ?? -99"# => "1",
);

test_run!(
    remove_inline_coalesce,
    r#"let d = ~{ a = 1 };"#,
    r#"d.remove("a") ?? -99"# => "1",
);

// insert's value param is anon-linked to the receiver's value type
test_fail!(
    insert_rejects_wrong_value_type,
    r#"fn take(d: ~{int}) { d.insert("b", "oops"); } let d: ~{int} = ~{ a = 1 }; take(d);"#
);
