// parse-time diagnostics, miette-derived. each kind carries its own `NamedSource` (cloned
// from the parser's per-file source) and a `SourceSpan` (translated from a `Location` at the
// throw site).
//
// the one chompy holdover is [`LexError`] -- a thin bridge wrapping chompy's `DiagBox`
// (still produced by the lex-helper trait the lexer impls). goes away when the lexer is
// rewritten off chompy.

use std::sync::Arc;

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

pub use shared::{Error, Result};

/// Bridge for chompy lex helper errors. The wrapped `DiagBox` carries the chompy-side
/// label/location info; we render its debug form as the diagnostic title. Goes away when
/// the lexer is rewritten off `chompy::lex::Lex`.
#[derive(Error, Debug, Diagnostic)]
#[error("{}", self.format())]
pub struct LexError {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    pub diag: chompy::diagnostics::DiagBox,
    // None when the diag's location is synthetic
    #[label("here")]
    pub at: Option<SourceSpan>,
}

impl LexError {
    fn format(&self) -> String {
        format!("{:?}", self.diag)
    }
}

#[derive(Error, Debug, Diagnostic)]
#[error("unterminated f-string expression")]
pub struct UnterminatedFStringExpr {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("this `{{` in an f-string was never closed with a `}}`")]
    pub at: SourceSpan,
}

/// Catch-all for syntax habits from other languages (`and`, `elif`, `as`, ...) -- caught
/// where a `;` was expected so the error teaches the mimas spelling instead of pointing at
/// a confusing semicolon.
#[derive(Error, Debug, Diagnostic)]
#[error("{msg}")]
pub struct Misdirection {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("{label}")]
    pub at: SourceSpan,
    pub msg: String,
    pub label: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("missing semicolon")]
pub struct MissingSemiColon {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("this statement needs to end with a `;`")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("expected identifier")]
pub struct ExpectedIdentifier {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("expected an identifier in this position")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("`?`/`!` can't constrain a single pact in a `+` bound")]
#[diagnostic(help("parenthesize the whole bound to apply it to every pact, e.g. `(Foo + Bar)?`"))]
pub struct PactBoundConstraint {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("this binds one pact, not the whole bound")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("only pacts can be joined with `+`")]
pub struct NonPactInBound {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("this `+` may only join pact names")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("parentheses around a single type do nothing")]
#[diagnostic(help("for a one-element tuple, add a trailing comma: `(T,)`"))]
pub struct SingleTypeParens {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("these parentheses are unnecessary")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("expected either")]
pub struct ExpectedPossibleTokens {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("expected this to be one of the following toks: {options}")]
    pub at: SourceSpan,
    pub options: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("doubled option")]
pub struct DoubledOption {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("options auto-flatten, so `T??` is just `T?` -- write that instead")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("nesting too deep")]
pub struct NestingTooDeep {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("code can nest at most 64 levels deep")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("unexpected token")]
pub struct UnexpectedToken {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("{tok} is not valid in this position")]
    pub at: SourceSpan,
    pub tok: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid assignment target")]
pub struct InvalidAssignmentTarget {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("this expression cannot be assigned a value")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("cannot glob-import all modules")]
pub struct UseAllModules {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("cannot glob-import all modules")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid dot access")]
pub struct InvalidDotAccess {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("only identifiers or integers can be used with dot access")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("named argument before positional")]
pub struct NamedBeforePositional {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("positional arguments must be provided prior to specifying titled parameters")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid impl item")]
pub struct InvalidImplItem {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("impl blocks can only declare constants and functions")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid pact item")]
pub struct InvalidPactItem {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("pacts can only declare constants and functions")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("missing function body")]
pub struct MissingFunctionBody {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("functions without bodies are not valid here")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("already in module")]
pub struct AlreadyInsideModule {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("cannot declare a second module within the same file")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("expected token")]
pub struct ExpectedToken {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("expected `{expected}` here")]
    pub at: SourceSpan,
    pub expected: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("parser misdirection")]
pub struct ParserMisdirection {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("the parser was confident `{expected}` would be here -- this is a bug!")]
    pub at: SourceSpan,
    pub expected: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("unexpected end of input")]
pub struct UnexpectedEnd {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("input ended here")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid pub marker")]
pub struct InvalidPubMarker {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("all members of pacts inherit the visibility of their pact by default")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("annotation required")]
pub struct PactConstAnnotationRequired {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("pacts must annotate the type of their const declarations")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("default parameter value not allowed in pact signature")]
pub struct PactSigDefaultParam {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("pact method signatures cannot give parameters default values")]
    pub at: SourceSpan,
}
