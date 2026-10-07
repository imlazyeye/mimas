use std::ops::Range;

use host::Library;
use serde::{Deserialize, Serialize};
use shared::Error;

use crate::{Cart, api::install::library};

/// A problem in one of a cart's files in the shape the page wants it. Lines and columns start at
/// 1, and a column counts UTF-16 code units, which is what a browser's text area counts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub message: String,
    pub label: Option<String>,
    pub help: Option<String>,
    pub kind: DiagnosticKind,
}

/// Whether a diagnostic is found before the cart runs or while it does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticKind {
    /// Found without running the cart, like a type error.
    Error,
    /// A runtime error that halted the machine.
    Fault,
}

impl Diagnostic {
    /// Shapes an error from the parser, the solver or the vm. Its first label gives the span, and
    /// the name of the label's source picks the file out of the cart. An error that points
    /// nowhere, or at a source that isn't one of the cart's files, is a point at the top of the
    /// cart's script.
    pub(crate) fn from_report(error: &Error, cart: &Cart, kind: DiagnosticKind) -> Self {
        let files = &cart.files;
        let label = error.labels().and_then(|mut labels| labels.next());
        let name = label.as_ref().and_then(|label| {
            let contents = error.source_code()?.read_span(label.inner(), 0, 0).ok()?;
            contents.name().map(str::to_owned)
        });
        let name = name
            .as_deref()
            .filter(|name| files.contains_key(*name))
            .or(cart.script().map(|(path, _)| path))
            .unwrap_or_default();
        let span = label
            .as_ref()
            .map_or(0..0, |label| label.offset()..label.offset() + label.len());
        let text = files.get(name).map_or("", String::as_str);

        let mut diagnostic = Self::at(name, text, span, error.to_string());
        diagnostic.label = label.and_then(|label| label.label().map(str::to_owned));
        diagnostic.help = error.help().map(|help| help.to_string());
        diagnostic.kind = kind;
        diagnostic
    }

    /// An error for the bytes of `text` in `span`, with no label or help. An empty span is a
    /// point. `text` is the file's whole text, so the columns are right however far into it the
    /// span is.
    pub fn at(file: &str, text: &str, span: Range<usize>, message: impl Into<String>) -> Self {
        /// The line and column of a byte offset, counted the way the page does.
        fn position(text: &str, mut offset: usize) -> (u32, u32) {
            offset = offset.min(text.len());
            while !text.is_char_boundary(offset) {
                offset -= 1;
            }
            let before = &text[..offset];
            let line = before.matches('\n').count() + 1;
            let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
            let col = before[line_start..].encode_utf16().count() + 1;
            (line as u32, col as u32)
        }

        let (line, col) = position(text, span.start);
        let (end_line, end_col) = position(text, span.end.max(span.start));
        Self {
            file: file.to_owned(),
            line,
            col,
            end_line,
            end_col,
            message: message.into(),
            label: None,
            help: None,
            kind: DiagnosticKind::Error,
        }
    }
}

/// Finds the problems in a cart without running it. Those are the rules it breaks (see
/// [`Cart::validate`]) followed by the compiler's errors for the modules and then for each script
/// on top of them.
pub fn check(cart: &Cart) -> Vec<Diagnostic> {
    check_with(cart, &library())
}

/// [`check`] against a [`library`] made earlier, for a caller that checks a cart over and over.
pub fn check_with(cart: &Cart, library: &Library<()>) -> Vec<Diagnostic> {
    let mut problems = cart.validate().err().unwrap_or_default();
    let modules = solve::Modules::from_files(cart.modules(), library);
    let scripts = cart.scripts().map(|script| modules.load([script]));
    let kind = DiagnosticKind::Error;
    for solved in std::iter::once(&modules).chain(&scripts.collect::<Vec<_>>()) {
        for error in &solved.errors {
            problems.push(Diagnostic::from_report(error, cart, kind));
        }
    }
    problems
}
