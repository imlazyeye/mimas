use line_index::{LineCol, LineIndex, TextSize, WideEncoding, WideLineCol};
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use parse::Ast;
use shared::Span;

/// One file of a project, with the line index that maps between its byte offsets and LSP
/// positions.
pub struct SourceFile {
    pub ast: Ast,
    lines: LineIndex,
}

impl SourceFile {
    pub fn new(text: &str, ast: Ast) -> Self {
        Self {
            ast,
            lines: LineIndex::new(text),
        }
    }

    pub fn diagnostic(&self, error: &shared::Error) -> Diagnostic {
        let label = error.labels().and_then(|mut labels| labels.next());
        let range = label
            .as_ref()
            .and_then(|label| self.range(Span::new(label.offset(), label.offset() + label.len())))
            .unwrap_or_default();
        let mut message = error.to_string();
        if let Some(text) = label.as_ref().and_then(|label| label.label()) {
            message.push('\n');
            message.push_str(text);
        }
        Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::Error),
            source: Some("mimas".to_owned()),
            message: message.into(),
            ..Default::default()
        }
    }

    /// A column past the end of the line clamps to the line end, as the protocol asks.
    pub fn offset(&self, position: Position) -> Option<usize> {
        let line = self.lines.line(position.line)?;
        let wide = WideLineCol {
            line: position.line,
            col: position.character,
        };
        let line_col = self.lines.to_utf8(WideEncoding::Utf16, wide)?;
        let offset = self.lines.offset(line_col)?;
        Some(offset.min(line.end()).into())
    }

    pub fn range(&self, span: Span) -> Option<Range> {
        Some(Range {
            start: self.position(span.start)?,
            end: self.position(span.end)?,
        })
    }

    fn position(&self, offset: usize) -> Option<Position> {
        let line_col: LineCol = self.lines.try_line_col(TextSize::new(offset as u32))?;
        let wide = self.lines.to_wide(WideEncoding::Utf16, line_col)?;
        Some(Position {
            line: wide.line,
            character: wide.col,
        })
    }
}
