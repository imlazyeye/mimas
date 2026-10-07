use crate::tests::utils::parse;

macro_rules! node_at_test {
    ($name:ident, $source:expr, $($needle:expr => $expected:expr),+ $(,)?) => {
        #[test]
        fn $name() {
            let source = $source;
            let (ast, errors) = parse(source);
            assert!(errors.is_empty(), "{errors:?}");
            $(
                let offset = source.find($needle).unwrap();
                let text = ast.node_at(offset).map(|(_, span)| &source[span.start..span.end]);
                assert_eq!(text, $expected, "node at {:?} in {source:?}", $needle);
            )+
        }
    };
}

node_at_test!(
    ident_inside_call,
    "let x = 1; print(x + 1);",
    "x +" => Some("x"),
);

node_at_test!(let_pattern, "let name = 1;", "name" => Some("name"));

node_at_test!(
    nested_expr_is_the_innermost,
    "let x = foo(a, bar(b));",
    "b)" => Some("b"),
    "bar" => Some("bar"),
);

node_at_test!(
    closure_parameter_and_body,
    "let f = |a| a + 1;",
    "a|" => Some("a"),
    "+" => Some("a + 1"),
);

node_at_test!(
    fn_body_and_parameter,
    "fn add(a: int, b: int) -> int { a + b }",
    "b }" => Some("b"),
    "a:" => Some("a"),
);

node_at_test!(whitespace_hits_nothing, "let x = 1;", " x" => None);

node_at_test!(
    function_matches_only_on_its_name,
    "fn add(a: int) -> int { a }",
    "add" => Some("add"),
    "fn" => None,
);

node_at_test!(
    struct_matches_only_on_its_name,
    "struct P { x: int }",
    "P" => Some("P"),
    "struct" => None,
);

node_at_test!(
    binding_and_struct_pattern_fields,
    "let whole @ Pair { first = value, second } = pair;",
    "whole" => Some("whole"),
    "first" => Some("first"),
    "value" => Some("value"),
    "second" => Some("second"),
);
