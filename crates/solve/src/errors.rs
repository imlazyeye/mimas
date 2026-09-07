use itertools::Itertools;
use miette::{Diagnostic, NamedSource, SourceSpan};
use std::sync::Arc;
use thiserror::Error;

pub use shared::{Error, Result};

#[derive(Error, Debug, Diagnostic)]
#[error("bare `null` binding")]
#[diagnostic(help("annotate the type so it can hold more than null, e.g. `let x: int? = null`"))]
pub struct BareNullBinding {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("`null` here can never be anything else, so this binding is useless")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("mismatched types")]
pub struct TypeMismatch {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("expected {expected} but found {found}")]
    pub at: SourceSpan,
    pub expected: String,
    pub found: String,
}

#[allow(unused)]
#[derive(Error, Debug, Diagnostic)]
#[error("cannot infer parameter type")]
pub struct CannotInferParameter {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("parameter '{name}' needs a type annotation, default value, or constraining use")]
    pub at: SourceSpan,
    pub name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("undefined variable")]
pub struct NotFound {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("could not find a value for '{name}' in scope")]
    pub at: SourceSpan,
    pub name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid `??` target")]
pub struct CoalesceNonOptional {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("left operand of `??` has type `{ty}`, which is never null")]
    pub at: SourceSpan,
    pub ty: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid pattern")]
pub struct InvalidPattern {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("pattern `{pattern}` cannot bind a value of type `{ty}`")]
    pub at: SourceSpan,
    pub pattern: String,
    pub ty: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("or-pattern alternatives bind different names")]
pub struct InconsistentOrBindings {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("`{name}` is not bound in all alternatives of this or-pattern")]
    pub at: SourceSpan,
    pub name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("if missing else clause")]
pub struct IfNeedsElse {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("if expressions without an else clause must evaluate to ()")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("let-else needs a diverging else")]
pub struct LetElseMustDiverge {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("this else must diverge -- return, break, continue, or panic")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid unwrap")]
pub struct InvalidUnwrap {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("attempted to unwrap {ty}, which is not an option")]
    pub at: SourceSpan,
    pub ty: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("collect outside loop")]
pub struct CollectOutsideLoop {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("cannot collect outside of a loop")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("break outside loop")]
pub struct BreakOutsideLoop {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("cannot break outside of a loop")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("break value with collection")]
pub struct BreakValueWhileCollection {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("cannot break a value in a loop also using `collect`")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("continue outside loop")]
pub struct ContinueOutOfLoop {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("cannot use `continue` outside of a loop")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("return outside function")]
pub struct ReturnOutOfFunction {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("cannot use `return` outside of a function")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("raise outside result-returning function")]
pub struct RaiseOutsideResult {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("`raise` is only valid in a function whose return type is annotated with `!`")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("absolve on non-result")]
pub struct AbsolveNonResult {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("`absolve` expects a result on the left, found `{ty}`")]
    pub at: SourceSpan,
    pub ty: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid evaluation")]
pub struct InvalidEvaluation {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("cannot use `{op}` with types `{lhs}` and `{rhs}`")]
    pub at: SourceSpan,
    pub lhs: String,
    pub rhs: String,
    pub op: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid unary operation")]
pub struct InvalidUnary {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("cannot use `{op}` with type `{ty}`")]
    pub at: SourceSpan,
    pub ty: String,
    pub op: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("not all paths return")]
pub struct NotAllPathsReturn {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("function body has paths that do not return '{ty}'")]
    pub at: SourceSpan,
    pub ty: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("not a struct")]
pub struct NotAStruct {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("'{ty}' is not a struct")]
    pub at: SourceSpan,
    pub ty: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("not an enum")]
pub struct NotAnEnum {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("'{ty}' is not an enum with variants")]
    pub at: SourceSpan,
    pub ty: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("not a pact")]
pub struct NotAPact {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("'{ty}' is not a pact")]
    pub at: SourceSpan,
    pub ty: String,
}

/// `s[0]` on a string. Its own error because the generic index path would otherwise report a
/// mismatch against a freshly minted element vid, putting an internal `?mimas<T>` in front of a
/// user who only wanted a character.
#[derive(Error, Debug, Diagnostic)]
#[error("strings can't be indexed")]
#[diagnostic(help("iterate the characters with `for c in s` instead"))]
pub struct StringIndexing {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("indexing is for arrays and dicts")]
    pub at: SourceSpan,
}

/// Reaching a pact *constant* -- rather than a method -- through the abstract pact type has no
/// lowering yet: each impl supplies a different value, and there's no dispatch for constants the
/// way there is for methods. Rejected here so it reads as a scope limit instead of surfacing as
/// an ICE further down the pipeline.
#[derive(Error, Debug, Diagnostic)]
#[error("pact constant `{member}` can't be reached through `{via}`")]
#[diagnostic(help(
    "constants don't dispatch the way pact methods do -- read it off a concrete type \
     (`TheType::{member}`), or add a pact method that returns it"
))]
pub struct PactConstantNotDispatchable {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("`{via}` is only known by its pact here, so the value isn't decided yet")]
    pub at: SourceSpan,
    pub member: String,
    pub via: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("'{member}' is declared by more than one pact in this bound")]
#[diagnostic(help("a `+` bound can't combine pacts that share a member name -- rename one"))]
pub struct AmbiguousPactMember {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("ambiguous across the bound's pacts")]
    pub at: SourceSpan,
    pub member: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("`{target}` already implements pact `{pact}`")]
pub struct DuplicatePactImpl {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("duplicate impl of `{pact}`")]
    pub at: SourceSpan,
    pub target: String,
    pub pact: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("enum cannot be constructed directly")]
pub struct EnumNotConstructable {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("pick a variant of `{name}`")]
    pub at: SourceSpan,
    pub name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("expected tuple struct")]
pub struct ExpectedTupleStruct {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("'{ty}' cannot be constructed as a tuple struct")]
    pub at: SourceSpan,
    pub ty: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("unrecognized field")]
pub struct FieldNotFound {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("no field or method named '{field_name}'")]
    pub at: SourceSpan,
    pub field_name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("out of bounds tuple read")]
pub struct OutOfBoundsTupleRead {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label(
        "tried to index tuple at index {tried_index}, but it only has {max_index} fields ({ty})"
    )]
    pub at: SourceSpan,
    pub tried_index: usize,
    pub max_index: usize,
    pub ty: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid tuple index")]
pub struct InvalidTupleIndex {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("tuples can only be dot-indexed with integers")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("not a tuple")]
pub struct NotATuple {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("only tuples may be indexed by integers")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid access")]
pub struct InvalidAccess {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("'{ty}' cannot be indexed")]
    pub at: SourceSpan,
    pub ty: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid in target")]
pub struct InvalidInTarget {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("cannot use `in` on `{ty}`")]
    pub at: SourceSpan,
    pub ty: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("variant not found")]
pub struct VariantNotFound {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("a variant called '{name}' was not found on enum")]
    pub at: SourceSpan,
    pub name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("missing fields")]
pub struct MissingStructFields {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("missing definitions for {}", self.fields.iter().join(", "))]
    pub at: SourceSpan,
    pub fields: Vec<String>,
}

#[derive(Error, Debug, Diagnostic)]
#[error("type has no fields")]
pub struct TypeHasNoFields {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("type '{ty}' does not have fields and therefore cannot be accessed")]
    pub at: SourceSpan,
    pub ty: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("not callable")]
pub struct NotCallable {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("'{ty}' cannot be called")]
    pub at: SourceSpan,
    pub ty: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("missing arguments")]
pub struct MissingArguments {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("call is missing required arguments")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("required parameter after optional")]
pub struct RequiredAfterOptional {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("once a parameter has a default, every later one must too")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("unknown named argument")]
pub struct UnknownNamedArgument {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("no parameter named '{name}'")]
    pub at: SourceSpan,
    pub name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("duplicate named argument")]
pub struct DuplicateNamedArgument {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("'{name}' was already supplied")]
    pub at: SourceSpan,
    pub name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("extra arguments")]
pub struct ExtraArguments {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("call has more arguments than possible")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("missing tuple members")]
pub struct MissingTupleMembers {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("tuple is missing required members")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("extra tuple members")]
pub struct ExtraTupleMembers {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("tuple has more members than possible")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("const declared multiple times")]
pub struct MultipleConstDeclarations {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("'{name}' is already defined as a constant")]
    pub at: SourceSpan,
    pub name: String,
    // the original may live in another file, so it gets its own sub-diagnostic
    #[related]
    pub original: Vec<ConstDefinedHere>,
}

#[derive(Error, Debug, Diagnostic)]
#[error("`{name}` defined here")]
#[diagnostic(severity(Advice))]
pub struct ConstDefinedHere {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("previous definition here")]
    pub at: SourceSpan,
    pub name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("impl item declared multiple times")]
pub struct DuplicateImplDeclaration {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("...with the name used here")]
    pub at: SourceSpan,
    #[label("'{name}' has already been declared")]
    pub original_at: SourceSpan,
    pub name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("cannot assign to constant value")]
pub struct AssignToConst {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("'{name}' is defined as a constant")]
    pub at: SourceSpan,
    pub name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("cannot assign to loop variable")]
pub struct AssignToLoopVar {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("'{name}' is a loop iterator and cannot be manually mutated")]
    pub at: SourceSpan,
    pub name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("non-constant value")]
pub struct NonConstantValue {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("'{value}' cannot be evaluated at compile time")]
    pub at: SourceSpan,
    pub value: String,
}

#[allow(unused)]
#[derive(Error, Debug, Diagnostic)]
#[error("value can be reduced")]
pub struct CanBeReduced {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("this expression can be reduced to '{reduction}'")]
    pub at: SourceSpan,
    pub reduction: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid impl target")]
pub struct InvalidImplTarget {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("impl can only be used on structs")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("self out of context")]
pub struct SelfOutOfContext {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("`self` can only be used inside a method that uses `self`")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid use target")]
pub struct InvalidUseTarget {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("this is not a module")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid assign target")]
pub struct InvalidAssignTarget {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("this cannot be assigned to")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid comparison")]
pub struct InvalidComparison {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("these two values cannot be compared like numerals")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("attempted to divide by zero")]
pub struct DivideByZero {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("this operation will panic at runtime")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("pacts are not values")]
pub struct PactIsNotAValue {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("a pact names a behavior, not a value -- bind a type that implements it instead")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("integer arithmetic overflows")]
pub struct ConstOverflow {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("this operation overflows a 64 bit integer")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("attempted to shift by an invalid integer")]
pub struct InvalidShift {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("bit shifts can only be done by unsigned 32 bit integers")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("cannot convert hex")]
pub struct InvalidHex {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("this cannot be converted into a 64 bit integer")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("`{name}` is private to its module")]
pub struct PrivateAccess {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("`{name}` is not visible from this module")]
    pub access_at: SourceSpan,
    pub name: String,
    // the declaration may live in another file, so it gets its own sub-diagnostic
    // (and source) instead of a second label on `src`
    #[related]
    pub declaration: Vec<PrivateDeclaredHere>,
}

#[derive(Error, Debug, Diagnostic)]
#[error("`{name}` defined here")]
#[diagnostic(severity(Advice))]
pub struct PrivateDeclaredHere {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("declared here without `pub`")]
    pub at: SourceSpan,
    pub name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("nested `fn` declaration")]
#[diagnostic(help(
    "use a closure (`let {name} = |...| ...`) -- mimas only supports `fn` at module scope"
))]
pub struct NestedFn {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("`fn` can only be declared at the top level")]
    pub at: SourceSpan,
    pub name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid iterator target")]
pub struct InvalidIterTarget {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("cannot iterate over `{ty}` -- expected an array, dict, str, or int")]
    pub at: SourceSpan,
    pub ty: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("non-constant default value")]
pub struct NonConstDefault {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("parameter defaults must be knowable at compile time")]
    pub at: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("unresolved constant `{name}`")]
#[diagnostic(help("constants can't depend on themselves, directly or through a cycle"))]
pub struct UnresolvedConst {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("could not reduce `{name}` to a constant value")]
    pub at: SourceSpan,
    pub name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("non-exhaustive match")]
pub struct NonExhaustiveMatch {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("{summary}")]
    pub at: SourceSpan,
    pub summary: String,
    #[help]
    pub help: Option<String>,
    #[related]
    pub variant_defs: Vec<VariantDefinedHere>,
}

#[derive(Error, Debug, Diagnostic)]
#[error("`{name}` defined here")]
#[diagnostic(severity(Advice))]
pub struct VariantDefinedHere {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("this variant has no matching arm")]
    pub at: SourceSpan,
    pub name: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("unrecognized pact constant")]
pub struct PactConstNotFound {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("'{pact}' does not have a constant named '{name}'")]
    pub at: SourceSpan,
    pub name: String,
    pub pact: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("unrecognized pact function")]
pub struct PactFunctionNotFound {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("'{pact}' does not have a function named '{name}'")]
    pub at: SourceSpan,
    pub name: String,
    pub pact: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("`{target}` does not fully fulfill its pact `{pact}`")]
#[diagnostic(help("missing: {missing}"))]
pub struct PactImplIncomplete {
    #[source_code]
    pub src: NamedSource<Arc<str>>,
    #[label("this impl is missing required members")]
    pub at: SourceSpan,
    pub target: String,
    pub pact: String,
    pub missing: String,
}

/// stop a giant type/value string from flooding a diagnostic label
pub(crate) fn elide(s: String) -> String {
    const MAX: usize = 100;
    if s.chars().count() <= MAX {
        s
    } else {
        format!("{}...", s.chars().take(MAX).collect::<String>())
    }
}
