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
            let lexer = $crate::lex::Lexer::new($src, 0, "test".into());
            let outputed = lexer
                .map(|tok| tok.unwrap().kind().clone())
                .collect::<Vec<$crate::lex::TokKind>>();
            pretty_assertions::assert_eq!(*outputed, expected)
        }
    };
}

#[cfg(test)]
pub(crate) fn assert_parses(source: &'static str) {
    let lexer = crate::lex::Lexer::new(source, 0, "test".into());
    let parsed = crate::Parser::new(lexer).into_ast();
    assert!(
        parsed.as_ref().is_ok_and(|v| v.stmts().len() == 1),
        "`{source}` should parse to one stmt but did not ({parsed:?})"
    );
}

#[cfg(test)]
pub(crate) fn assert_rejects(source: &'static str) {
    let lexer = crate::lex::Lexer::new(source, 0, "test".into());
    let parsed = crate::Parser::new(lexer).into_ast();
    assert!(
        parsed.map_or(true, |v| v.stmts().len() != 1),
        "`{source}` should have been rejected but parsed"
    );
}

#[macro_export]
macro_rules! test_ok {
    ($name:ident, $($src:expr),+ $(,)?) => {
        #[cfg(test)]
        #[test]
        fn $name() {
            $( $crate::tests::utils::assert_parses($src); )+
        }
    };
}

#[macro_export]
macro_rules! test_fail {
    ($name:ident, $($src:expr),+ $(,)?) => {
        #[cfg(test)]
        #[test]
        fn $name() {
            $( $crate::tests::utils::assert_rejects($src); )+
        }
    };
}
