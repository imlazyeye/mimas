use crate::tests::utils::parse;

fn docs_at(source: &str, name: &str) -> Option<String> {
    let (ast, errors) = parse(source);
    assert!(errors.is_empty(), "{errors:?}");
    ast.docs_for(source.find(name).unwrap()).map(str::to_string)
}

#[test]
fn doc_is_at_the_name_it_is_for() {
    let source = "
    /// A point.
    struct Point {
        /// Across.
        pub x: int,
    }
    /// A shape.
    enum Shape {
        /// Round.
        Circle,
    }
    /// Things with names.
    pact Named {
        /// The name.
        fn name(self) -> str;
    }
    /// Adds.
    pub fn add() {}
    /// The start.
    let origin = 0;";
    for (name, doc) in [
        ("Point", "A point."),
        ("x:", "Across."),
        ("Shape", "A shape."),
        ("Circle", "Round."),
        ("Named", "Things with names."),
        ("name(", "The name."),
        ("add(", "Adds."),
        ("origin", "The start."),
    ] {
        assert_eq!(docs_at(source, name).as_deref(), Some(doc), "{name}");
    }
}

#[test]
fn doc_lines_lose_their_slashes() {
    let source = "/// Adds two numbers.
                  ///
                  /// Returns the sum.
                  fn add() {}";
    let docs = docs_at(source, "add(").unwrap();
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
    assert_eq!(docs_at(source, "foo").as_deref(), Some("b"));
}

#[test]
fn plain_comment_between_doc_and_item() {
    let source = "/// Docs.
                  // todo
                  fn foo() {}";
    assert_eq!(docs_at(source, "foo").as_deref(), Some("Docs."));
}

#[test]
fn plain_comment_is_not_a_doc() {
    let source = "// not docs
                  fn foo() {}";
    assert_eq!(docs_at(source, "foo"), None);
}
