use line_index::{LineCol, LineIndex, TextSize, WideEncoding, WideLineCol};
use lsp_types::{Diagnostic, DiagnosticSeverity, DocumentSymbol, Position, Range, SymbolKind};
use parse::{
    Ast, FieldKey, Ident, Item, ItemKind, Member, NodeId, PactItem, Stmt, StmtKind, StructField,
    Visitor, walk_stmts,
};
use shared::{Located, Span};

/// One file of a project, with the line index that maps between its byte offsets and LSP
/// positions.
pub struct SourceFile {
    pub text: String,
    pub ast: Ast,
    lines: LineIndex,
}

impl SourceFile {
    pub fn new(text: String, ast: Ast) -> Self {
        Self {
            lines: LineIndex::new(&text),
            text,
            ast,
        }
    }

    /// Every ident written in the file, with the span it was written at.
    pub fn idents(&self) -> Vec<(NodeId, Span)> {
        #[derive(Default)]
        struct Idents(Vec<(NodeId, Span)>);

        impl Visitor for Idents {
            fn ident(&mut self, ident: &Ident) {
                self.0.push((ident.id, ident.location.span));
            }
        }

        let mut idents = Idents::default();
        walk_stmts(self.ast.stmts(), &mut idents);
        idents.0
    }

    /// The file's outline -- its items, their members, and its top level bindings. Reads only the
    /// ast, so it still answers while the project doesn't type check.
    pub fn symbols(&self) -> Vec<DocumentSymbol> {
        // `deprecated` is deprecated in favor of `tags`, but the struct still requires it, for some
        // reason?
        #[allow(deprecated)]
        fn symbol(
            file: &SourceFile,
            name: &Ident,
            kind: SymbolKind,
            range: Span,
        ) -> Option<DocumentSymbol> {
            Some(DocumentSymbol {
                name: name.lexeme.clone(),
                detail: None,
                kind,
                tags: None,
                deprecated: None,
                range: file.range(range)?,
                selection_range: file.range(name.location.span)?,
                children: None,
            })
        }

        fn item_symbol(file: &SourceFile, item: &Item) -> Option<DocumentSymbol> {
            let (name, kind, detail, children) = match item.kind() {
                ItemKind::Function(function) => {
                    (&function.name, SymbolKind::Function, None, vec![])
                }
                ItemKind::Const(con) => (&con.left, SymbolKind::Constant, None, vec![]),
                ItemKind::Struct(struc) => {
                    let fields = struc
                        .fields
                        .iter()
                        .filter_map(|field| {
                            let FieldKey::Ident(name) = &field.name else {
                                return None;
                            };
                            symbol(file, name, SymbolKind::Field, name.location.span)
                        })
                        .collect();
                    (&struc.name, SymbolKind::Struct, None, fields)
                }
                ItemKind::Enum(en) => {
                    let variants = en
                        .members
                        .iter()
                        .filter_map(|(name, _)| {
                            symbol(file, name, SymbolKind::EnumMember, name.location.span)
                        })
                        .collect();
                    (&en.head, SymbolKind::Enum, None, variants)
                }
                ItemKind::Pact(pact) => {
                    let items = pact
                        .items
                        .iter()
                        .filter_map(|item| {
                            let (name, kind) = match item {
                                PactItem::Const { name, .. } => (name, SymbolKind::Constant),
                                PactItem::Fn { name, .. } => (name, SymbolKind::Method),
                            };
                            symbol(file, name, kind, name.location.span)
                        })
                        .collect();
                    (&pact.name, SymbolKind::Interface, None, items)
                }
                ItemKind::Impl(imp) => {
                    let methods = imp
                        .items
                        .iter()
                        .filter_map(|item| {
                            let mut symbol = item_symbol(file, item)?;
                            if matches!(symbol.kind, SymbolKind::Function) {
                                symbol.kind = SymbolKind::Method;
                            }
                            Some(symbol)
                        })
                        .collect();
                    let detail = match &imp.pact {
                        Some(pact) => format!("impl {}", pact.lexeme),
                        None => "impl".to_owned(),
                    };
                    (&imp.target, SymbolKind::Class, Some(detail), methods)
                }
                ItemKind::Use(_) | ItemKind::Poison(_) => return None,
            };
            let mut symbol = symbol(file, name, kind, item.span())?;
            symbol.detail = detail;
            symbol.children = (!children.is_empty()).then_some(children);
            Some(symbol)
        }

        self.ast
            .stmts()
            .iter()
            .filter_map(|stmt| match stmt.kind() {
                StmtKind::Item(item) => item_symbol(self, item),
                StmtKind::Let(binding) => {
                    let name = binding.left.as_ident()?;
                    symbol(self, name, SymbolKind::Variable, stmt.span())
                }
                _ => None,
            })
            .collect()
    }

    /// The doc comment on the declaration whose name is written at `name`.
    pub fn docs(&self, name: Span) -> Option<&str> {
        /// Where the declaration named at `name` starts (its doc is keyed there).
        struct Declaration {
            name: Span,
            start: Option<usize>,
        }

        impl Declaration {
            fn check(&mut self, name: &Ident, start: usize) {
                if name.location.span == self.name {
                    self.start = Some(start);
                }
            }

            fn fields(&mut self, fields: &[StructField]) {
                for field in fields {
                    if let FieldKey::Ident(name) = &field.name {
                        self.check(name, field.location.span.start);
                    }
                }
            }
        }

        impl Visitor for Declaration {
            fn stmt(&mut self, stmt: &Stmt) {
                if let StmtKind::Let(binding) = stmt.kind()
                    && let Some(name) = binding.left.as_ident()
                {
                    self.check(name, stmt.span().start);
                }
            }

            fn item(&mut self, item: &Item) {
                let start = item.span().start;
                match item.kind() {
                    ItemKind::Function(function) => self.check(&function.name, start),
                    ItemKind::Const(con) => self.check(&con.left, start),
                    ItemKind::Struct(struc) => {
                        self.check(&struc.name, start);
                        self.fields(&struc.fields);
                    }
                    ItemKind::Enum(en) => {
                        self.check(&en.head, start);
                        for (name, member) in &en.members {
                            self.check(name, name.location.span.start);
                            if let Member::Struct(fields) = member {
                                self.fields(fields);
                            }
                        }
                    }
                    ItemKind::Pact(pact) => {
                        self.check(&pact.name, start);
                        for member in &pact.items {
                            let (PactItem::Const { name, .. } | PactItem::Fn { name, .. }) = member;
                            self.check(name, member.span().start);
                        }
                    }
                    // the walk visits impl methods as items of their own
                    ItemKind::Impl(_) | ItemKind::Use(_) | ItemKind::Poison(_) => {}
                }
            }
        }

        let mut declaration = Declaration { name, start: None };
        walk_stmts(self.ast.stmts(), &mut declaration);
        self.ast.docs_for(declaration.start?)
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
