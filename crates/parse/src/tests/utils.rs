#[macro_export]
macro_rules! tok {
    ($name:ident) => {
        chompy::lex::Token::lazy(parse::lex::TokKind::$name)
    };
}

#[macro_export]
macro_rules! tok_test {
    ($name:ident: $src:expr => $($should_be:expr), * $(,)?) => {
        #[cfg(test)]
        #[test]
        fn $name() {
            use chompy::lex::Token;
            let expected = vec![$($should_be, )*];
            let mut lexer = $crate::lex::Lexer::new($src, 0, "test".into());
            let outputed = lexer
                .by_ref()
                .map(|tok| tok.kind().clone())
                .collect::<Vec<$crate::lex::TokKind>>();
            let errors = lexer.take_errors();
            assert!(errors.is_empty(), "`{}` reported {:?}", $src, errors);
            pretty_assertions::assert_eq!(*outputed, expected)
        }
    };
}

/// Parses `source` into whatever Ast comes out, along with every error.
#[cfg(test)]
pub(crate) fn parse(source: &str) -> (crate::Ast, Vec<shared::Error>) {
    let lexer = crate::lex::Lexer::new(source, 0, "test".into());
    crate::Parser::new(lexer).into_ast()
}

/// Parses `source`, giving back each top-level stmt and error as a string.
#[cfg(test)]
pub(crate) fn parse_to_strings(source: &str) -> (Vec<String>, Vec<String>) {
    let (ast, errors) = parse(source);
    (
        ast.stmts().iter().map(ToString::to_string).collect(),
        errors.iter().map(ToString::to_string).collect(),
    )
}

/// The text `err`'s first label points at.
#[cfg(test)]
pub(crate) fn label_text<'a>(source: &'a str, err: &shared::Error) -> &'a str {
    let label = err.labels().unwrap().next().unwrap();
    &source[label.offset()..label.offset() + label.len()]
}

#[cfg(test)]
pub(crate) fn assert_parses(source: &str) {
    let (stmts, errors) = parse_to_strings(source);
    assert!(
        errors.is_empty() && stmts.len() == 1,
        "`{source}` should parse to one stmt but did not ({stmts:?}, {errors:?})"
    );
}

#[cfg(test)]
pub(crate) fn assert_rejects(source: &str) {
    let (stmts, errors) = parse_to_strings(source);
    assert!(
        !errors.is_empty() || stmts.len() != 1,
        "`{source}` should have been rejected but parsed"
    );
}

#[macro_export]
macro_rules! test_ok {
    ($name:ident, $($src:expr),+ $(,)?) => {
        #[cfg(test)]
        #[test]
        fn $name() {
            $( $crate::tests::utils::assert_parses(&$src); )+
        }
    };
}

#[macro_export]
macro_rules! test_fail {
    ($name:ident, $($src:expr),+ $(,)?) => {
        #[cfg(test)]
        #[test]
        fn $name() {
            $( $crate::tests::utils::assert_rejects(&$src); )+
        }
    };
}
