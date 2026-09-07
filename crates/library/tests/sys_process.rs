#[macro_use]
mod test_runner;

// std::process::run -- happy paths
test_run!(
    process_run_echo_hello,
    r#"std::process::run("echo", ["hello"], null)!"# => r#""hello\n""#
);

test_run!(
    process_run_echo_uses_use_alias,
    "use std::process;",
    r#"process::run("echo", ["hi"], null)!"# => r#""hi\n""#
);

test_run!(
    process_run_multiple_args_space_joined,
    r#"std::process::run("echo", ["a", "b", "c"], null)!"# => r#""a b c\n""#
);

test_run!(
    process_run_empty_args,
    r#"std::process::run("echo", [], null)!"# => r#""\n""#
);

test_run!(
    process_run_result_len,
    r#"std::process::run("echo", ["hello"], null)!.len()"# => "6"
);

test_run!(
    process_run_result_trim,
    r#"std::process::run("echo", ["hello"], null)!.trim()"# => r#""hello""#
);

// stdin piping -- `cat` echoes whatever we feed it
test_run!(
    process_run_stdin_piped_to_cat,
    r#"std::process::run("cat", [], "piped input")!"# => r#""piped input""#
);

test_run!(
    process_run_stdin_empty_string,
    r#"std::process::run("cat", [], "")!"# => r#""""#
);

test_run!(
    process_run_stdin_multiline,
    r#"std::process::run("cat", [], "a\nb\n")!"# => r#""a\nb\n""#
);

// non-zero exit -> raised; recover with absolve
test_run!(
    process_run_false_nonzero_exit_is_raised,
    r#"std::process::run("false", [], null) absolve |_| "ABSOLVED""# => r#""ABSOLVED""#
);

test_run!(
    process_run_nonzero_exit_error_mentions_cmd,
    r#""false" in (std::process::run("false", [], null) absolve |e| e)"# => "true"
);

// spawn failure (binary not found) -> raised
test_run!(
    process_run_missing_binary_is_raised,
    r#"std::process::run("definitely_not_a_real_cmd_xyz_123", [], null) absolve |_| "NOPE""#
        => r#""NOPE""#
);

// non-zero exit unwrapped with `!` faults the VM
test_fail!(
    process_run_unwrapped_faults,
    r#"let TEST_VALUE = std::process::run("false", [], null)!;"#,
    r#"let TEST_VALUE = std::process::run("definitely_not_a_real_cmd_xyz_123", [], null)!;"#
);

// the result is `str!`: accessing it without unwrapping is a type error (no fields on a result)
test_fail!(
    process_run_result_needs_unwrap_before_field_access,
    r#"let TEST_VALUE = std::process::run("echo", ["x"], null).len();"#
);

// absolve handler return type must match the result inner type (str)
test_fail!(
    process_run_absolve_handler_type_must_be_str,
    r#"let TEST_VALUE = std::process::run("echo", ["x"], null) absolve |_| 0;"#
);

// arg types: cmd is str, args is [str], stdin is str?; wrong arity rejected
test_fail!(
    process_run_arg_type_and_arity_checks,
    r#"let TEST_VALUE = std::process::run(42, [], null)!;"#,
    r#"let TEST_VALUE = std::process::run("echo", [1, 2], null)!;"#,
    r#"let TEST_VALUE = std::process::run("echo", [], 5)!;"#,
    r#"let TEST_VALUE = std::process::run("echo")!;"#
);

// std::sys::arg -- in the test harness ScriptArgs is never set, so every index is null
test_run!(
    sys_arg_unset_is_null,
    r#"std::sys::arg(0) == null"# => "true",
    r#"std::sys::arg(99) == null"# => "true"
);

test_run!(
    sys_arg_unwrap_or_default,
    r#"let a = std::sys::arg(0);"#,
    r#"if a == null { "fallback" } else { a! }"# => r#""fallback""#
);

test_run!(
    sys_arg_use_alias,
    "use std::sys;",
    r#"sys::arg(0) == null"# => "true"
);

// arg type checks
test_fail!(
    sys_arg_type_and_arity_checks,
    r#"let TEST_VALUE = std::sys::arg("0");"#,
    r#"let TEST_VALUE = std::sys::arg();"#,
    r#"let TEST_VALUE: str = std::sys::arg(0);"#
);

// std::sys::exit and std::sys::stdin are intentionally NOT exercised in a passing test:
// exit() kills the test process and stdin() blocks reading from the harness's stdin.
// Type-shape only: stdin() is a `str!` result, exit(code) returns the never type `!`.
test_fail!(
    sys_stdin_exit_type_shape,
    r#"let TEST_VALUE = std::sys::stdin().len();"#,
    r#"let TEST_VALUE = std::sys::exit("nope");"#
);
