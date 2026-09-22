use crate::tests::utils::parse;

fn node_text_at(source: &str, offset: usize) -> Option<String> {
    let (ast, errors) = parse(source);
    assert!(errors.is_empty(), "{errors:?}");
    let (_, span) = ast.node_at(offset)?;
    Some(source[span.start..span.end].to_string())
}

#[test]
fn ident_inside_call() {
    let source = "let x = 1; print(x + 1);";
    let x = source.rfind('x').unwrap();
    assert_eq!(node_text_at(source, x).as_deref(), Some("x"));
}

#[test]
fn let_pattern() {
    let source = "let name = 1;";
    assert_eq!(node_text_at(source, 4).as_deref(), Some("name"));
}

#[test]
fn nested_expr_is_the_innermost() {
    let source = "let x = foo(a, bar(b));";
    let b = source.rfind('b').unwrap();
    assert_eq!(node_text_at(source, b).as_deref(), Some("b"));
    let bar = source.find("bar").unwrap();
    assert_eq!(node_text_at(source, bar).as_deref(), Some("bar"));
}

#[test]
fn closure_parameter_and_body() {
    let source = "let f = |a| a + 1;";
    let param = source.find('a').unwrap();
    assert_eq!(node_text_at(source, param).as_deref(), Some("a"));
    let plus = source.find('+').unwrap();
    assert_eq!(node_text_at(source, plus).as_deref(), Some("a + 1"));
}

#[test]
fn fn_body_and_parameter() {
    let source = "fn add(a: int, b: int) -> int { a + b }";
    let b = source.rfind('b').unwrap();
    assert_eq!(node_text_at(source, b).as_deref(), Some("b"));
    let param = source.find("a:").unwrap();
    assert_eq!(node_text_at(source, param).as_deref(), Some("a"));
}

#[test]
fn whitespace_hits_nothing() {
    assert_eq!(node_text_at("let x = 1;", 3), None);
}

#[test]
fn item_matches_only_on_its_name() {
    let source = "fn add(a: int) -> int { a }";
    assert_eq!(node_text_at(source, 3).as_deref(), Some("add"));
    assert_eq!(node_text_at(source, 0), None);
    let source = "struct P { x: int }";
    assert_eq!(node_text_at(source, 7).as_deref(), Some("P"));
}
