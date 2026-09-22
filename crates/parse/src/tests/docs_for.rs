use crate::tests::utils::parse;
use shared::Located;

fn docs_at(source: &str, token: &str) -> Option<String> {
    let (ast, errors) = parse(source);
    assert!(errors.is_empty(), "{errors:?}");
    ast.docs_for(source.find(token).unwrap())
        .map(str::to_string)
}

#[test]
fn doc_is_at_the_start_of_its_item() {
    let source = "/// Adds.
                  pub fn add() {}";
    let (ast, errors) = parse(source);
    assert!(errors.is_empty(), "{errors:?}");
    let start = ast.stmts()[0].span().start;
    assert_eq!(ast.docs_for(start), Some("Adds."));
}

#[test]
fn doc_lines_lose_their_slashes() {
    let source = "/// Adds two numbers.
                  ///
                  /// Returns the sum.
                  fn add() {}";
    let docs = docs_at(source, "fn").unwrap();
    assert_eq!(
        docs.lines().collect::<Vec<_>>(),
        ["Adds two numbers.", "", "Returns the sum."]
    );
}

#[test]
fn only_the_closest_doc_counts() {
    let source = "/// a

                  /// b
                  fn foo() {}";
    assert_eq!(docs_at(source, "fn").as_deref(), Some("b"));
}

#[test]
fn plain_comment_between_doc_and_item() {
    let source = "/// Docs.
                  // todo
                  fn foo() {}";
    assert_eq!(docs_at(source, "fn").as_deref(), Some("Docs."));
}

#[test]
fn plain_comment_is_not_a_doc() {
    let source = "// not docs
                  fn foo() {}";
    assert_eq!(docs_at(source, "fn"), None);
}
