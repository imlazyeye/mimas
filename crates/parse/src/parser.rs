use crate::{components::*, errors::*, *};
use bitflags::bitflags;
use chompy::{
    lex::{Lex, Tok, Token},
    utils::Located as _,
};
use lex::{Lexer, TokKind};
use miette::NamedSource;
use shared::{FileId, Located, Location, Result, Span};
use std::{iter::Peekable, sync::Arc};

/// Recursively descends mimas source, incrementally returning various
/// statements and expressions.
pub struct Parser<'s> {
    lexer: Peekable<Lexer<'s>>,
    cursor: usize,
    file_id: FileId,
    file_name: String,
    src: NamedSource<Arc<str>>,
    restrictions: Restriction,
    depth: usize,
}

// Basic features
impl<'s> Parser<'s> {
    /// Creates a new parser.
    pub fn new(lexer: Lexer<'s>) -> Self {
        let file_id = lexer.file_id();
        let file_name: String = lexer.file_name().into();
        let src = NamedSource::new(file_name.clone(), Arc::<str>::from(lexer.source()));
        let mut lexer = lexer.peekable();
        let cursor = lexer
            .peek()
            .map_or(0, |v| v.as_ref().map_or(0, |v| v.span().start()));
        Self {
            file_id,
            lexer,
            cursor,
            file_name,
            src,
            restrictions: Restriction::default(),
            depth: 0,
        }
    }

    /// Guards the recursive descent against stack overflow: deeply-nested (or unbalanced --
    /// 50k stray `(`s) input would otherwise SIGABRT before any diagnostic. Callers pair this
    /// with a `self.depth -= 1` after the recursive body returns; the error path skips the
    /// decrement because the whole parse aborts on the first error anyway.
    fn descend(&mut self) -> Result<()> {
        const MAX_DEPTH: usize = 64;
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            let at = shared::Location::from(self.peek()?.location()).into();
            Err(NestingTooDeep {
                src: self.src(),
                at,
            })?;
        }
        Ok(())
    }

    /// Clone the per-file `NamedSource` for embedding in a diagnostic. Cheap (Arc + String).
    pub(crate) fn src(&self) -> NamedSource<Arc<str>> {
        self.src.clone()
    }

    /// Runs the parser through the entire source, collecting everything into an
    /// Ast and returning it.
    ///
    /// ### Errors
    ///
    /// Returns a [ParseError] if any of the source code caused an error.
    pub fn into_ast(mut self) -> Result<Ast> {
        let mut statements = vec![];
        while self.soft_peek()?.is_some() {
            statements.push(self.stmt()?);
        }
        Ok(Ast::new(self.file_name, self.src, statements))
    }

    /// Creates a new expression.
    fn new_expr(&self, expr: impl Into<ExprKind>, start: usize) -> Expr {
        Expr::new(expr.into(), Location::new(self.file_id, self.span(start)))
    }

    /// Creates a new statement.
    fn new_stmt(&self, stmt: impl Into<StmtKind>, start: usize) -> Stmt {
        Stmt::new(stmt.into(), NodeId::new(), self.location(start))
    }

    /// Creates a [Span] from the given position up until our current position.
    fn span(&self, start: usize) -> Span {
        Span::new(start, self.cursor)
    }

    /// Creates a [Location] from the given position up until our current position.
    fn location(&self, start: usize) -> Location {
        Location::new(self.file_id, self.span(start))
    }
}

// Recursive descent (mimas grammar)
impl<'s> Parser<'s> {
    pub(crate) fn stmt(&mut self) -> Result<Stmt> {
        let start = self.next_tok_boundary();
        match self.node()? {
            BlockElement::Stmt(stmt) => Ok(stmt),
            BlockElement::MaybeYield(expr) => {
                let stmt = self.new_stmt(StmtKind::Expr(expr), start);
                self.semicolon_check(&stmt)?;
                Ok(stmt)
            }
        }
    }

    /// Routes the next token to the right parse function and reports back what kind of thing came
    /// out: a fully-formed statement, or a bare expression whose role (yielded value vs expression
    /// statement) the caller decides. Assignments finish here as `Stmt`; only true bare exprs
    /// surface as `MaybeYield`.
    fn node(&mut self) -> Result<BlockElement> {
        let start = self.next_tok_boundary();
        match self.peek()?.kind() {
            TokKind::Let => self.let_stmt().map(BlockElement::Stmt),
            TokKind::Module => self.module_stmt().map(BlockElement::Stmt),
            TokKind::Pub
            | TokKind::Fn
            | TokKind::Struct
            | TokKind::Pact
            | TokKind::Enum
            | TokKind::Impl
            | TokKind::Const
            | TokKind::Use => {
                let item = self.item()?;
                let stmt = self.new_stmt(item, start);
                Ok(BlockElement::Stmt(stmt))
            }
            // here to give a nicer diagnostic than "expected identifier" when the user writes `;;`
            TokKind::SemiColon => {
                let tok = self.take()?;
                Err(self.unexpected(tok).into())
            }
            _ => {
                let expr = self.expr()?;

                // TODO: assignment does not belong here... I think
                if let Some(operator) = self.soft_peek()?.and_then(|tok| tok.kind().try_into().ok())
                {
                    // Ensure the left is a valid assignment target
                    if !matches!(expr.kind(), ExprKind::Access(_) | ExprKind::Ident(_)) {
                        Err(errors::InvalidAssignmentTarget {
                            src: self.src(),
                            at: expr.location().into(),
                        })?
                    } else {
                        self.take()?;
                        let assignment = Assignment::new(expr, operator, self.expr()?);
                        let stmt = self.new_stmt(assignment, start);
                        self.ok_if_semicolon(stmt).map(BlockElement::Stmt)
                    }
                } else {
                    Ok(BlockElement::MaybeYield(expr))
                }
            }
        }
    }

    fn item(&mut self) -> Result<Item> {
        let start = self.next_tok_boundary();
        let public = self.match_take(TokKind::Pub).is_some();

        let (kind, requires_semi): (ItemKind, bool) = match self.peek()?.kind() {
            TokKind::Fn => (self.function()?.into(), false),
            TokKind::Struct => (self.struct_decl()?.into(), false),
            TokKind::Pact => (self.pact_decl()?.into(), false),
            TokKind::Enum => (self.enum_decl()?.into(), false),
            TokKind::Impl => (self.impl_decl()?.into(), false),
            TokKind::Const => (self.const_decl()?.into(), true),
            TokKind::Use => (self.use_decl()?.into(), true),
            _ => {
                let tok = self.take()?;
                Err(self.unexpected(tok))?
            }
        };

        let location = self.location(start);
        if requires_semi {
            if self.match_take(TokKind::SemiColon).is_none() {
                Err(MissingSemiColon {
                    src: self.src(),
                    at: location.into(),
                })?
            }
        } else {
            // optional trailing `;` after fn/struct/enum/impl
            self.match_take(TokKind::SemiColon);
        }

        Ok(Item::new(kind, location, public))
    }

    fn use_decl(&mut self) -> Result<Use> {
        self.take_known(TokKind::Use)?;

        // detailed error, mostly as a humorous nod to rustc
        if let Some(tok) = self.match_take(TokKind::Star) {
            Err(UseAllModules {
                src: self.src(),
                at: shared::Location::from(tok.location()).into(),
            })?;
        }

        // All use statements must start with an identifier
        let mut path = vec![self.require_ident()?];

        // Now a semi colon ends us or a colon continues us
        loop {
            let tok = self.peek()?;
            match tok.kind() {
                TokKind::SemiColon => {
                    let item = path.pop().unwrap();
                    return Ok(Use::Singular(path, item));
                }
                TokKind::DoubleColon => {
                    self.take()?;
                    match self.peek()?.kind() {
                        TokKind::Ident(_) => {
                            path.push(self.require_ident()?);
                        }
                        TokKind::Star => {
                            self.take()?;
                            return Ok(Use::All(path));
                        }
                        TokKind::LeftBrace => {
                            self.take()?;
                            let mut items = vec![];
                            while let Some(ident) = self.match_take_ident()? {
                                items.push(ident);
                                if self.match_take(TokKind::Comma).is_none() {
                                    break;
                                }
                                if self.peek()?.kind() == TokKind::RightBrace {
                                    break;
                                }
                            }
                            self.expect(TokKind::RightBrace)?;
                            return Ok(Use::Multi(path, items));
                        }
                        _ => {
                            let tok = self.take()?;
                            Err(self.unexpected(tok))?
                        }
                    }
                }
                _ => {
                    let tok = self.take()?;
                    Err(self.unexpected(tok))?
                }
            }
        }
    }

    fn module_stmt(&mut self) -> Result<Stmt> {
        // First we take the module
        let start = self.next_tok_boundary();
        self.take_known(TokKind::Module)?;

        // We either are given an ident or an @, which maps to the file name. The lexer's name
        // can be a full path (that's what diagnostics render), so take the stem here.
        let module = if self.match_take(TokKind::At).is_some() {
            let stem = std::path::Path::new(&self.file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&self.file_name)
                .to_string();
            Module::new(Ident::new(stem, self.location(start)))
        } else {
            // `module a::b;` -- nested path, flattened into one `::`-joined name (the solver
            // splits it back out through ensure_module_path)
            let mut ident = self.require_ident()?;
            while self.match_take(TokKind::DoubleColon).is_some() {
                let segment = self.require_ident()?;
                ident.lexeme = format!("{}::{}", ident.lexeme, segment.lexeme);
            }
            ident.location = self.location(start);
            Module::new(ident)
        };

        let stmt = self.new_stmt(module, start);
        self.ok_if_semicolon(stmt)
    }

    fn let_stmt(&mut self) -> Result<Stmt> {
        let start = self.next_tok_boundary();
        self.take_known(TokKind::Let)?;
        let left = self.pattern()?;
        let annotation = if self.match_take(TokKind::Colon).is_some() {
            Some(self.annotation()?)
        } else {
            None
        };
        self.expect(TokKind::Equal)?;
        let right = self.expr()?;
        let else_branch = if self.match_take(TokKind::Else).is_some() {
            Some(self.expr()?)
        } else {
            None
        };
        let stmt = self.new_stmt(
            Let {
                left,
                annotation,
                right,
                else_branch,
            },
            start,
        );
        self.ok_if_semicolon(stmt)
    }

    fn const_decl(&mut self) -> Result<Const> {
        self.take_known(TokKind::Const)?;
        let left = self.require_ident()?;
        let annotation = if self.match_take(TokKind::Colon).is_some() {
            Some(self.annotation()?)
        } else {
            None
        };
        self.expect(TokKind::Equal)?;
        let right = self.expr()?;
        Ok(Const {
            left,
            annotation,
            right,
        })
    }

    /// Parses the source mimas for a new expression.
    ///
    ///  ### Errors
    ///
    /// Returns a [ParseError] if any of the source code caused an error.
    pub fn expr(&mut self) -> Result<Expr> {
        self.descend()?;
        let result = self
            .block_body()
            .and_then(|expr| self.chain_after_block(expr));
        self.depth -= 1;
        result
    }

    /// The expression dispatch without chaining a trailing block. Control-flow *bodies* parse
    /// through here so postfix after the body (`if c {...} else {...}.foo()`) attaches to the whole
    /// construct, not the inner block; `expr` wraps this with `chain_after_block`.
    fn block_body(&mut self) -> Result<Expr> {
        match self.peek()?.kind() {
            TokKind::Pipe | TokKind::DoublePipe => self.closure(),
            TokKind::Loop => self.loop_expr(),
            TokKind::While => self.while_expr(),
            TokKind::For => self.for_in(),
            TokKind::If => self.if_expr(),
            TokKind::Match => self.match_expr(),
            TokKind::LeftBrace => self.block(),
            TokKind::Return => self.return_expr(),
            TokKind::Raise => self.raise_expr(),
            TokKind::Break => self.break_stmt(),
            TokKind::Collect => self.collect(),
            TokKind::Continue => self.continue_expr(),

            // No keyword expressions found, so start recursive descent
            _ => self.null_coalecence(),
        }
    }

    fn struct_decl(&mut self) -> Result<Struct> {
        self.take_known(TokKind::Struct)?;
        let name = self.require_ident()?;
        let mut fields = vec![];

        if self.match_take(TokKind::SemiColon).is_none() {
            if self.match_take(TokKind::LeftParenthesis).is_some() {
                let mut iter = 0;
                loop {
                    if self.match_take(TokKind::RightParenthesis).is_some() {
                        break;
                    } else {
                        let start = self.next_tok_boundary();
                        let public = self.match_take(TokKind::Pub).is_some();
                        let annotation = self.annotation()?;
                        fields.push(StructField {
                            name: FieldKey::Int(iter),
                            annotation,
                            public,
                            location: Location::new(self.file_id, self.span(start)),
                        });
                        iter += 1;
                        if self.match_take(TokKind::Comma).is_none() {
                            self.expect(TokKind::RightParenthesis)?;
                            break;
                        }
                    }
                }
            } else {
                self.expect(TokKind::LeftBrace)?;
                loop {
                    if self.match_take(TokKind::RightBrace).is_some() {
                        break;
                    } else {
                        let start = self.next_tok_boundary();
                        let public = self.match_take(TokKind::Pub).is_some();
                        let name = self.require_ident()?;
                        self.expect(TokKind::Colon)?;
                        let annotation = self.annotation()?;
                        fields.push(StructField {
                            name: FieldKey::Ident(name),
                            annotation,
                            public,
                            location: Location::new(self.file_id, self.span(start)),
                        });
                        if self.match_take(TokKind::Comma).is_none() {
                            self.expect(TokKind::RightBrace)?;
                            break;
                        }
                    }
                }
            }
        }

        Ok(Struct { name, fields })
    }

    fn pact_decl(&mut self) -> Result<Pact> {
        self.take_known(TokKind::Pact)?;
        let pact_name = self.require_ident()?;
        self.expect(TokKind::LeftBrace)?;
        let mut items = Vec::new();
        loop {
            if self.match_take(TokKind::RightBrace).is_some() {
                break;
            }
            if let Some(tok) = self.match_take(TokKind::Pub) {
                return Err(InvalidPubMarker {
                    src: self.src(),
                    at: shared::Location::from(tok.location()).into(),
                }
                .into());
            }
            let (item, requires_semi) = match self.peek()?.kind() {
                TokKind::Const => {
                    self.take_known(TokKind::Const)?;
                    let name = self.require_ident()?;
                    if self.match_take(TokKind::Colon).is_none() {
                        let location = self.peek()?.location();
                        return Err(PactConstAnnotationRequired {
                            src: self.src(),
                            at: shared::Location::from(location).into(),
                        }
                        .into());
                    }
                    let annotation = self.annotation()?;
                    (PactItem::Const { name, annotation }, true)
                }
                TokKind::Fn => {
                    let (name, parameters, return_type) = self.function_sig()?;
                    for param in &parameters {
                        if let Some(default) = &param.right {
                            return Err(PactSigDefaultParam {
                                src: self.src(),
                                at: default.location().into(),
                            }
                            .into());
                        }
                    }
                    // optional default body -- `{ ... }` after the sig. when present, the trailing
                    // `;` is dropped (block-terminated, like normal fns).
                    let default = if self.peek().is_ok_and(|t| t.kind() == TokKind::LeftBrace) {
                        Some(self.block()?)
                    } else {
                        None
                    };
                    let requires_semi = default.is_none();
                    (
                        PactItem::Fn {
                            name,
                            parameters,
                            return_type,
                            default,
                        },
                        requires_semi,
                    )
                }
                _ => {
                    let location = self.peek()?.location();
                    return Err(InvalidPactItem {
                        src: self.src(),
                        at: shared::Location::from(location).into(),
                    }
                    .into());
                }
            };
            if self.match_take(TokKind::SemiColon).is_none() && requires_semi {
                let location = self.peek()?.location();
                return Err(MissingSemiColon {
                    src: self.src(),
                    at: shared::Location::from(location).into(),
                }
                .into());
            }
            items.push(item);
        }
        Ok(Pact::new(pact_name, items))
    }

    fn enum_decl(&mut self) -> Result<Enum> {
        self.take_known(TokKind::Enum)?;
        let name = self.require_ident()?;
        let mut members = vec![];
        self.expect(TokKind::LeftBrace)?;
        loop {
            if self.match_take(TokKind::RightBrace).is_some() {
                break;
            } else {
                let name = self.require_ident()?;
                if self.match_take(TokKind::LeftParenthesis).is_some() {
                    let mut tuple_members = vec![];

                    while self.match_take(TokKind::RightParenthesis).is_none() {
                        tuple_members.push(self.annotation()?);
                        if self.match_take(TokKind::Comma).is_none() {
                            self.expect(TokKind::RightParenthesis)?;
                            break;
                        }
                    }

                    members.push((name, Member::Tuple(tuple_members)));
                } else {
                    let mut fields = vec![];
                    if self.match_take(TokKind::LeftBrace).is_some() {
                        loop {
                            if self.match_take(TokKind::RightBrace).is_some() {
                                break;
                            }
                            let field_start = self.next_tok_boundary();
                            let field = self.require_ident()?;
                            self.expect(TokKind::Colon)?;
                            let annotation = self.annotation()?;
                            fields.push(StructField {
                                name: FieldKey::Ident(field),
                                annotation,
                                location: Location::new(self.file_id, self.span(field_start)),
                                public: true,
                            });
                            if self.match_take(TokKind::Comma).is_none() {
                                self.expect(TokKind::RightBrace)?;
                                break;
                            }
                        }
                    }
                    members.push((name, Member::Struct(fields)));
                }
            }
            if self.match_take(TokKind::Comma).is_none() {
                self.expect(TokKind::RightBrace)?;
                break;
            }
        }

        Ok(Enum {
            head: name,
            members,
        })
    }

    fn impl_decl(&mut self) -> Result<Impl> {
        let start = self.next_tok_boundary();
        self.take_known(TokKind::Impl)?;
        let first = self.require_ident()?;
        // `impl Foo {}` (inherent) vs `impl Pact for Foo {}` (pact impl). After the first ident,
        // a `for` token disambiguates -- first becomes the pact, second becomes the target.
        let (pact, target) = if self.match_take(TokKind::For).is_some() {
            let target = self.require_ident()?;
            (Some(first), target)
        } else {
            (None, first)
        };
        self.expect(TokKind::LeftBrace)?;
        let mut items = Vec::new();
        loop {
            if self.match_take(TokKind::RightBrace).is_some() {
                break;
            } else {
                let item_start = self.next_tok_boundary();
                let public = self.match_take(TokKind::Pub).is_some();
                let (kind, requires_semi): (ItemKind, bool) = match self.peek()?.kind() {
                    TokKind::Const => (self.const_decl()?.into(), true),
                    TokKind::Fn => (self.function()?.into(), false),
                    _ => {
                        return Err(InvalidImplItem {
                            src: self.src(),
                            at: self.location(start).into(),
                        }
                        .into());
                    }
                };
                let location = self.location(item_start);
                if requires_semi {
                    if self.match_take(TokKind::SemiColon).is_none() {
                        Err(MissingSemiColon {
                            src: self.src(),
                            at: location.into(),
                        })?
                    }
                } else {
                    self.match_take(TokKind::SemiColon);
                }
                items.push(Item::new(kind, location, public));
            }
        }
        Ok(Impl::new(target, pact, items))
    }

    fn function_sig(&mut self) -> Result<(Ident, Vec<Binding>, Option<Annotation>)> {
        self.take_known(TokKind::Fn)?;
        let name = self.require_ident()?;
        self.expect(TokKind::LeftParenthesis)?;
        let mut parameters = vec![];
        let mut method = false;
        loop {
            if self.match_take(TokKind::RightParenthesis).is_some() {
                break;
            } else if !method && let Some(s) = self.match_take(TokKind::SelfKeyword) {
                method = true;
                self.match_take(TokKind::Comma);
                let ident = Ident {
                    lexeme: "self".into(),
                    location: s.location.into(),
                };
                parameters.push(Binding::new(ident));
            } else {
                let name = self.require_ident()?;
                let mut binding = Binding::new(name);
                if self.match_take(TokKind::Colon).is_some() {
                    binding.annotation = Some(self.annotation()?);
                }
                if self.match_take(TokKind::Equal).is_some() {
                    binding.right = Some(self.expr()?);
                }
                parameters.push(binding);
                if self.match_take(TokKind::Comma).is_none() {
                    self.expect(TokKind::RightParenthesis)?;
                    break;
                }
            }
        }
        let return_type = if self.match_take(TokKind::Arrow).is_some() {
            Some(self.annotation()?)
        } else {
            None
        };

        Ok((name, parameters, return_type))
    }

    fn function(&mut self) -> Result<Function> {
        let start = self.next_tok_boundary();
        let (name, parameters, return_type) = self.function_sig()?;
        let body = if self.peek()?.kind() == TokKind::LeftBrace {
            self.block()?
        } else {
            Err(MissingFunctionBody {
                src: self.src(),
                at: self.location(start).into(),
            })?
        };

        Ok(Function {
            name,
            parameters,
            return_type,
            body,
        })
    }

    fn closure(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        if self.match_take(TokKind::DoublePipe).is_some() {
            let body = self.expr()?;

            let return_type = if self.match_take(TokKind::Arrow).is_some() {
                Some(self.annotation()?)
            } else {
                None
            };

            Ok(self.new_expr(
                Closure {
                    parameters: vec![],
                    body,
                    return_type,
                },
                start,
            ))
        } else {
            self.expect(TokKind::Pipe)?;
            let mut parameters = vec![];
            loop {
                if self.match_take(TokKind::Pipe).is_some() {
                    break;
                } else {
                    let name = self.require_ident()?;
                    let mut binding = Binding::new(name);
                    if self.match_take(TokKind::Colon).is_some() {
                        binding.annotation = Some(self.annotation()?);
                    }
                    parameters.push(binding);
                    if self.match_take(TokKind::Comma).is_none() {
                        self.expect(TokKind::Pipe)?;
                        break;
                    }
                }
            }
            let return_type = if self.match_take(TokKind::Arrow).is_some() {
                Some(self.annotation()?)
            } else {
                None
            };

            let body = self.expr()?;

            Ok(self.new_expr(
                Closure {
                    parameters,
                    body,
                    return_type,
                },
                start,
            ))
        }
    }

    fn break_stmt(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        self.take_known(TokKind::Break)?;
        let expr = self.optional_expr();
        Ok(self.new_expr(Break::new(expr), start))
    }

    fn collect(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        self.take_known(TokKind::Collect)?;
        let expr = self.expr()?;
        Ok(self.new_expr(Collect::new(expr), start))
    }

    fn continue_expr(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        self.take_known(TokKind::Continue)?;
        Ok(self.new_expr(ExprKind::Continue(Continue), start))
    }

    fn return_expr(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        self.take_known(TokKind::Return)?;
        let expr = self.optional_expr();
        Ok(self.new_expr(Return::new(expr), start))
    }

    fn raise_expr(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        self.take_known(TokKind::Raise)?;
        let value = self.expr()?;
        Ok(self.new_expr(Raise::new(value), start))
    }

    fn loop_expr(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        self.take_known(TokKind::Loop)?;
        let body = self.block_body()?;
        Ok(self.new_expr(Loop::new(body), start))
    }

    fn while_expr(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        self.take_known(TokKind::While)?;
        let binding = if self.match_take(TokKind::Let).is_some() {
            let binding = self.pattern()?;
            self.expect(TokKind::Equal)?;
            Some(binding)
        } else {
            None
        };
        self.restrictions.insert(Restriction::NO_STRUCT_LITERAL);
        let header = self.expr()?;
        self.restrictions.remove(Restriction::NO_STRUCT_LITERAL);
        if let Some(err) = self.condition_misdirection(binding.is_none()) {
            return Err(err);
        }
        let body = self.block_body()?;
        Ok(self.new_expr(
            While {
                header,
                body,
                binding,
            },
            start,
        ))
    }

    fn for_in(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        self.take_known(TokKind::For)?;

        let binding = self.pattern()?;
        self.expect(TokKind::In)?;
        self.restrictions.insert(Restriction::NO_STRUCT_LITERAL);

        let iterator = self.expr()?;

        let iterator = if let Some(dot) =
            self.match_take_possibilities(&[TokKind::DoubleDot, TokKind::DoubleDotEqual])
        {
            let start = iterator;
            let start_boundary = start.span().start();
            let end = self.expr()?;
            self.new_expr(
                Range::new(start, end, dot.kind() == TokKind::DoubleDotEqual),
                start_boundary,
            )
        } else {
            iterator
        };

        self.restrictions.remove(Restriction::NO_STRUCT_LITERAL);
        let body = self.block_body()?;
        Ok(self.new_expr(For::new(binding, iterator, body), start))
    }

    fn if_expr(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        self.take_known(TokKind::If)?;
        let binding = if self.match_take(TokKind::Let).is_some() {
            let binding = self.pattern()?;
            self.expect(TokKind::Equal)?;
            Some(binding)
        } else {
            None
        };
        self.restrictions.insert(Restriction::NO_STRUCT_LITERAL);
        let condition = self.expr()?;
        self.restrictions.remove(Restriction::NO_STRUCT_LITERAL);
        if let Some(err) = self.condition_misdirection(binding.is_none()) {
            return Err(err);
        }
        let main_body = self.block_body()?;
        let else_expr = if self.match_take(TokKind::Else).is_some() {
            let expr = self.block_body()?;
            Some(expr)
        } else {
            None
        };
        Ok(self.new_expr(
            If {
                condition,
                main_body,
                else_expr,
                binding,
            },
            start,
        ))
    }

    fn match_expr(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        self.take_known(TokKind::Match)?;
        self.restrictions.insert(Restriction::NO_STRUCT_LITERAL);
        let expr = self.expr()?;
        self.restrictions.remove(Restriction::NO_STRUCT_LITERAL);
        self.expect(TokKind::LeftBrace)?;
        let mut members = vec![];
        let mut panic_terminator = false;
        loop {
            if self.match_take(TokKind::RightBrace).is_some() {
                break;
            }
            if self.match_take(TokKind::Bang).is_some() {
                panic_terminator = true;
                self.match_take(TokKind::Comma);
                self.expect(TokKind::RightBrace)?;
                break;
            }
            let pattern = self.pattern()?;
            let guard = if self.match_take(TokKind::If).is_some() {
                Some(self.expr()?)
            } else {
                None
            };
            self.expect(TokKind::FatArrow)?;
            let body_is_block = matches!(self.peek()?.kind(), TokKind::LeftBrace);
            let body = self.expr()?;
            members.push(MatchCase::new(pattern, guard, body));
            if self.match_take(TokKind::Comma).is_none() {
                // block-bodied arms can omit the trailing comma; otherwise the case is the last
                if !body_is_block {
                    self.expect(TokKind::RightBrace)?;
                    break;
                }
            }
        }
        Ok(self.new_expr(Match::new(expr, members, panic_terminator), start))
    }

    fn null_coalecence(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        let expr = self.logical()?;
        if self.match_take(TokKind::DoubleHook).is_some() {
            let value = self.expr()?;
            Ok(self.new_expr(Coalescence::new(expr, value), start))
        } else if self.match_take(TokKind::Absolve).is_some() {
            let handler = self.expr()?;
            Ok(self.new_expr(Absolve::new(expr, handler), start))
        } else {
            Ok(expr)
        }
    }

    fn logical(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        let mut expr = self.equality()?;
        while let Some(operator) = self.soft_peek()?.and_then(|tok| tok.kind().try_into().ok()) {
            self.take()?;
            let right = self.equality()?;
            expr = self.new_expr(Logical::new(expr, operator, right), start);
        }
        Ok(expr)
    }

    fn equality(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        let mut expr = self.membership()?;
        while let Some(operator) = self.soft_peek()?.and_then(|tok| tok.kind().try_into().ok()) {
            self.take()?;
            let right = self.membership()?;
            expr = self.new_expr(Equality::new(expr, operator, right), start);
        }
        Ok(expr)
    }

    fn membership(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        let expr = self.evaluation()?;
        let condition = if self.match_take(TokKind::In).is_some() {
            true
        } else if self.match_take(TokKind::NotIn).is_some() {
            false
        } else {
            return Ok(expr);
        };

        let right = self.evaluation()?;
        Ok(self.new_expr(In::new(expr, right, condition), start))
    }

    fn evaluation(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        let mut expr = self.bitshift()?;
        while let Some(operator) = self
            .soft_peek()?
            .and_then(|tok| tok.kind().try_into().ok())
            .filter(EvaluationOp::is_binary)
        {
            self.take()?;
            let right = self.bitshift()?;
            expr = self.new_expr(Evaluation::new(expr, operator, right), start);
        }
        Ok(expr)
    }

    fn bitshift(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        let mut expr = self.addition()?;
        while let Some(operator) = self
            .soft_peek()?
            .and_then(|tok| tok.kind().try_into().ok())
            .filter(EvaluationOp::is_bit_shift)
        {
            self.take()?;
            let right = self.addition()?;
            expr = self.new_expr(Evaluation::new(expr, operator, right), start);
        }
        Ok(expr)
    }

    fn addition(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        let mut expr = self.multiplication()?;
        while let Some(operator) = self
            .soft_peek()?
            .and_then(|tok| tok.kind().try_into().ok())
            .filter(EvaluationOp::is_additive)
        {
            self.take()?;
            let right = self.multiplication()?;
            expr = self.new_expr(Evaluation::new(expr, operator, right), start);
        }
        Ok(expr)
    }

    fn multiplication(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        let mut expr = self.unary()?;
        while let Some(operator) = self
            .soft_peek()?
            .and_then(|tok| tok.kind().try_into().ok())
            .filter(EvaluationOp::is_multiplicative)
        {
            self.take()?;
            let right = self.unary()?;
            expr = self.new_expr(Evaluation::new(expr, operator, right), start);
        }
        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        if let Ok(operator) = self.peek()?.kind().try_into() {
            self.take()?;
            let right = self.unary()?;
            Ok(self.new_expr(Unary::new(operator, right), start))
        } else {
            self.fstring()
        }
    }

    fn fstring(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        if let TokKind::FString(content) = self.peek()?.kind() {
            // byte offset of the f-string's content in the outer source -- used to align spans
            // emitted by the sub-parser we spawn for each interp body. token span covers `f"..."`,
            // so content begins at span.start() + 2 (skip `f"`).
            let fstring_span_start = self.peek()?.location().span().start();
            let content_offset = fstring_span_start + 2;
            self.take()?;

            let mut parts = vec![];
            let mut current_literal = String::new();
            let mut chars = content.char_indices().peekable();
            while let Some((_, c)) = chars.next() {
                if c == '\\' {
                    current_literal.push(c);
                    if let Some((_, next)) = chars.next() {
                        current_literal.push(next);
                    }
                } else if c == '{' && matches!(chars.peek(), Some(&(_, '{'))) {
                    // `{{` escape -> literal `{`
                    chars.next();
                    current_literal.push('{');
                } else if c == '}' && matches!(chars.peek(), Some(&(_, '}'))) {
                    // `}}` escape -> literal `}`
                    chars.next();
                    current_literal.push('}');
                } else if c == '{' {
                    if !current_literal.is_empty() {
                        // `{` and `}` join the quote-chars so chompy's unescape resolves
                        // `\{` -> `{` and `\}` -> `}` (the brace-escape form, sibling to
                        // `{{`/`}}`).
                        let segment =
                            chompy::utils::unescape(&current_literal, &['\\'], &['"', '{', '}']);
                        parts.push(FStringPart::Literal(segment));
                        current_literal.clear();
                    }
                    // absolute byte offset (in outer source) of the first char inside the interp
                    let body_start =
                        content_offset + chars.peek().map_or(content.len(), |(i, _)| *i);
                    let mut expr_str = String::new();
                    let mut depth = 1;
                    let mut closed = false;
                    while let Some((_, c)) = chars.next() {
                        if c == '{' {
                            depth += 1;
                            expr_str.push(c);
                        } else if c == '}' {
                            depth -= 1;
                            if depth == 0 {
                                closed = true;
                                break;
                            }
                            expr_str.push(c);
                        } else if c == '"' {
                            // nested string literal in the interp expression -- pass
                            // it through verbatim so a `}` inside the string doesn't
                            // close the interp.
                            expr_str.push(c);
                            let mut esc = false;
                            for (_, sc) in chars.by_ref() {
                                expr_str.push(sc);
                                if esc {
                                    esc = false;
                                } else if sc == '\\' {
                                    esc = true;
                                } else if sc == '"' {
                                    break;
                                }
                            }
                        } else {
                            expr_str.push(c);
                        }
                    }
                    if !closed {
                        Err(UnterminatedFStringExpr {
                            src: self.src(),
                            at: self.location(self.cursor).into(),
                        })?;
                    }
                    // literal insanity -- we offset the string with spaces so that diagnostics
                    // point to the correct spans relative to our real lexer, lol, this is bad
                    //
                    // i mean it literally caused stack overflows in the lexer and i had to rewrite
                    // it to no longer recurse on each whitespace
                    let padded = format!("{}{}", " ".repeat(body_start), expr_str);
                    let lexer = Lexer::new(&padded, self.file_id, self.file_name.clone());
                    let mut parser = Parser::new(lexer);
                    let expr = parser.expr()?;
                    parts.push(FStringPart::Expr(expr));
                } else {
                    current_literal.push(c);
                }
            }
            if !current_literal.is_empty() {
                let segment = chompy::utils::unescape(&current_literal, &['\\'], &['"', '{', '}']);
                parts.push(FStringPart::Literal(segment));
            }

            let expr = self.new_expr(FString::new(parts), start);
            self.chain_accesses(expr)
        } else {
            self.literal()
        }
    }

    fn literal(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        if let Ok(literal) = Literal::try_from(self.peek()?.kind()) {
            self.take()?;
            let expr = self.new_expr(literal, start);

            // todo: this might allow "hello"() or true[]" etc. only dot accesses are okay on lits"
            self.chain_accesses(expr)
        } else if self.match_take(TokKind::LeftSquare).is_some() {
            let mut elements = vec![];
            let expr = loop {
                if self.match_take(TokKind::RightSquare).is_some() {
                    let literal = Literal::Array(elements);
                    break self.new_expr(literal, start);
                } else {
                    elements.push(self.expr()?);
                    self.match_take(TokKind::Comma);
                }
            };
            self.chain_accesses(expr)
        } else if self.match_take(TokKind::TildeLeftBrace).is_some() {
            let mut elements = vec![];
            let expr = loop {
                if self.match_take(TokKind::RightBrace).is_some() {
                    let literal = Literal::Dictionary(elements);
                    break self.new_expr(literal, start);
                } else {
                    let name = self.require_ident()?;
                    self.expect(TokKind::Equal)?;
                    elements.push((name, self.expr()?));
                    if self.match_take(TokKind::Comma).is_none() {
                        let _tok = self.expect(TokKind::RightBrace)?;
                        let literal = Literal::Dictionary(elements);
                        break self.new_expr(literal, start);
                    }
                }
            };
            self.chain_accesses(expr)
        } else {
            let expr = self.supreme()?;
            if !self.restrictions.contains(Restriction::NO_STRUCT_LITERAL)
                && self.match_take(TokKind::LeftBrace).is_some()
            {
                let mut fields = vec![];
                loop {
                    if self.match_take(TokKind::RightBrace).is_some() {
                        break;
                    } else {
                        self.next_tok_boundary();
                        let iden = self.require_ident()?;
                        if self.match_take(TokKind::Comma).is_some()
                            || self.peek()?.kind == TokKind::RightBrace
                        {
                            let this_start = iden.location.span().start();
                            fields.push((
                                FieldKey::Ident(iden.clone()),
                                self.new_expr(iden, this_start),
                            ));
                        } else {
                            self.expect(TokKind::Equal)?;
                            let expr = self.expr()?;
                            fields.push((FieldKey::Ident(iden), expr));
                            if self.match_take(TokKind::Comma).is_none() {
                                if self.match_take(TokKind::RightBrace).is_some() {
                                    break;
                                }
                                self.expect(TokKind::Comma)?;
                            }
                        }
                    }
                }

                let expr =
                    self.new_expr(Literal::Struct(StructLiteral { name: expr, fields }), start);
                self.chain_accesses(expr)
            } else {
                Ok(expr)
            }
        }
    }

    fn supreme(&mut self) -> Result<Expr> {
        let expr = self.call(None)?;
        self.chain_accesses(expr)
    }

    /// Chains any trailing access expressions (`.field`, `[key]`, `::member`,
    /// `()` calls, `!` unwraps, bare `?` postfix) onto an already-parsed expression.
    fn chain_accesses(&mut self, expr: Expr) -> Result<Expr> {
        let mut expr = Some(expr);
        loop {
            expr = match self.soft_peek()?.map(|v| v.kind()) {
                Some(TokKind::LeftParenthesis) => Some(self.call(expr)?),
                Some(TokKind::LeftSquare | TokKind::HookLeftSquare) => {
                    Some(self.square_access(expr)?)
                }
                Some(TokKind::DoubleColon) => Some(self.colon_access(expr)?),
                Some(TokKind::Dot | TokKind::HookDot) => Some(self.dot_access(expr)?),
                Some(TokKind::Bang) => Some(self.unwrap(expr)?),
                // bare trailing ?'s do nothing but are permitted. future warning
                Some(TokKind::Hook) => {
                    self.match_take(TokKind::Hook);
                    expr
                }
                _ => break Ok(expr.unwrap()),
            }
        }
    }

    /// Postfix chaining for block expressions. `(` and `[` directly after a block are ambiguous
    /// with a following statement (`if c {}` newline `[x]`), so they need parens; `.`/`::`/`!`/`?`
    /// can't start a statement, so once one attaches we hand off to the full `chain_accesses`.
    fn chain_after_block(&mut self, expr: Expr) -> Result<Expr> {
        match self.soft_peek()?.map(|t| t.kind()) {
            Some(
                TokKind::Dot
                | TokKind::HookDot
                | TokKind::DoubleColon
                | TokKind::Bang
                | TokKind::Hook,
            ) => self.chain_accesses(expr),
            _ => Ok(expr),
        }
    }

    fn unwrap(&mut self, left: Option<Expr>) -> Result<Expr> {
        let (start, left) = if let Some(left) = left {
            (left.span().start(), left)
        } else {
            let start = self.next_tok_boundary();
            let call = self.call(None)?;
            if !matches!(self.soft_peek()?.map(|t| t.kind()), Some(TokKind::Bang)) {
                return Ok(call);
            } else {
                (start, call)
            }
        };
        self.take_known(TokKind::Bang)?;
        Ok(self.new_expr(Unwrap { expr: left }, start))
    }

    fn call(&mut self, left: Option<Expr>) -> Result<Expr> {
        // If we've been provided a leftside expression, we *must* parse for a call.
        // Otherwise, the call is merely possible.
        let (start, left) = if let Some(left) = left {
            (left.span().start(), left)
        } else {
            let start = self.next_tok_boundary();
            let dot = self.dot_access(None)?;
            if !matches!(
                self.soft_peek()?.map(|t| t.kind()),
                Some(TokKind::LeftParenthesis)
            ) {
                return Ok(dot);
            }
            (start, dot)
        };
        self.expect(TokKind::LeftParenthesis)?;
        let mut arguments = vec![];
        let mut positional_allowed = true;
        while self.match_take(TokKind::RightParenthesis).is_none() {
            let expr = self.expr()?;
            let (name, value) = if let Some(tok) = self.match_take(TokKind::Equal) {
                let ExprKind::Ident(ident) = expr.kind() else {
                    Err(self.unexpected(tok))?
                };
                let assigned_value = self.expr()?;
                positional_allowed = false;
                (Some(ident.clone()), assigned_value)
            } else if !positional_allowed {
                return Err(NamedBeforePositional {
                    src: self.src(),
                    at: expr.location().into(),
                }
                .into());
            } else {
                (None, expr)
            };
            arguments.push(Argument { name, value });
            self.match_take(TokKind::Comma);
        }

        Ok(self.new_expr(Call::new(left, arguments), start))
    }

    fn dot_access(&mut self, left: Option<Expr>) -> Result<Expr> {
        fn read<'s>(parser: &mut Parser<'s>, start: usize) -> Result<Expr> {
            match parser.peek()?.kind() {
                TokKind::Ident(_) => {
                    let ident = parser.require_ident()?;
                    Ok(parser.new_expr(ident, start))
                }
                // take the int token directly -- `parser.literal()` would chain further accesses,
                // which would steal a trailing `.foo()` from the *outer* dot (`a.0.pairs()` would
                // misparse as `a . (0.pairs())`). The outer chain_accesses loop owns chaining.
                TokKind::Int(_) => {
                    let tok = parser.take()?;
                    let literal = Literal::try_from(tok.kind()).unwrap();
                    Ok(parser.new_expr(literal, start))
                }
                _ => Err(InvalidDotAccess {
                    src: parser.src(),
                    at: parser.location(start).into(),
                })?,
            }
        }
        let mut start = self.next_tok_boundary();
        let access = if let Some(left) = left {
            let tok = self.require_possibilities(&[TokKind::Dot, TokKind::HookDot])?;
            start = left.span().start();
            let right = read(self, start)?;

            Access::Dot {
                left,
                right,
                kind: tok.kind().try_into().unwrap(),
            }
        } else {
            let left = self.colon_access(None)?;
            if let Some(tok) = self.match_take_possibilities(&[TokKind::Dot, TokKind::HookDot]) {
                let right = read(self, start)?;

                Access::Dot {
                    left,
                    right,
                    kind: tok.kind().try_into().unwrap(),
                }
            } else {
                return Ok(left);
            }
        };
        Ok(self.new_expr(access, start))
    }

    fn colon_access(&mut self, expr: Option<Expr>) -> Result<Expr> {
        let mut start = self.next_tok_boundary();
        let access = if let Some(expr) = expr {
            self.expect(TokKind::DoubleColon)?;
            start = expr.span().start();
            let right = self.require_member_name()?;
            Access::DoubleColon { left: expr, right }
        } else if let Ok(TokKind::TyKw(ty)) = self.peek().map(|t| t.kind()) {
            // `int::random` etc. -- synthesize an Ident at expr head so the regular library map
            // lookup handles it. TyKw is only legal here in `TyKw ::` shape.
            let tok = self.take()?;
            self.expect(TokKind::DoubleColon)?;
            let left = self.new_expr(Ident::new(ty.to_string(), tok.location().into()), start);
            let right = self.require_member_name()?;
            Access::DoubleColon { left, right }
        } else {
            let left = self.square_access(None)?;
            if self.match_take(TokKind::DoubleColon).is_some() {
                let right = self.require_member_name()?;
                Access::DoubleColon { left, right }
            } else {
                return Ok(left);
            }
        };
        Ok(self.new_expr(access, start))
    }

    fn square_access(&mut self, left: Option<Expr>) -> Result<Expr> {
        let (start, left) = if let Some(left) = left {
            (left.span().start(), left)
        } else {
            let left = self.parentheticals()?;
            if !matches!(
                self.soft_peek()?.map(|v| v.kind()),
                Some(TokKind::LeftSquare | TokKind::HookLeftSquare)
            ) {
                return Ok(left);
            }
            (self.next_tok_boundary(), left)
        };
        let tok = self.require_possibilities(&[TokKind::LeftSquare, TokKind::HookLeftSquare])?;
        let key = self.expr()?;
        let access = Access::Square {
            left,
            key,
            kind: tok.kind().try_into().unwrap(),
        };
        let _tok = self.expect(TokKind::RightSquare)?;
        Ok(self.new_expr(access, start))
    }

    fn parentheticals(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        if self.match_take(TokKind::LeftParenthesis).is_some() {
            if self.match_take(TokKind::RightParenthesis).is_some() {
                Ok(self.new_expr(Literal::Unit, start))
            } else {
                let expr = self.expr()?;
                if self.match_take(TokKind::Comma).is_some() {
                    let mut members = vec![expr];
                    while self.match_take(TokKind::RightParenthesis).is_none() {
                        members.push(self.expr()?);
                        if self.match_take(TokKind::Comma).is_none() {
                            self.expect(TokKind::RightParenthesis)?;
                            break;
                        }
                    }
                    Ok(self.new_expr(Literal::Tuple(members), start))
                } else {
                    self.expect(TokKind::RightParenthesis)?;
                    Ok(self.new_expr(Grouping::new(expr), start))
                }
            }
        } else {
            self.block()
        }
    }

    fn block(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        if self.match_take(TokKind::LeftBrace).is_some() {
            let mut body: Vec<Stmt> = vec![];
            let yielded_expr = loop {
                if self.match_take(TokKind::RightBrace).is_some() {
                    break None;
                }
                let node_start = self.next_tok_boundary();
                match self.node()? {
                    BlockElement::Stmt(stmt) => body.push(stmt),
                    BlockElement::MaybeYield(expr) => {
                        if self.match_take(TokKind::RightBrace).is_some() {
                            // last node in the block -- this is the yield, not a new stmt
                            break Some(expr);
                        }
                        let stmt = self.new_stmt(StmtKind::Expr(expr), node_start);
                        self.semicolon_check(&stmt)?;
                        body.push(stmt);
                    }
                }
            };
            Ok(self.new_expr(Block { body, yielded_expr }, start))
        } else {
            self.ident()
        }
    }

    fn ident(&mut self) -> Result<Expr> {
        let start = self.next_tok_boundary();
        // we're just gonna hijack this guy...
        let ident = if let Some(tok) = self.match_take(TokKind::SelfKeyword) {
            Ident {
                lexeme: tok.to_string(),
                location: self.location(start),
            }
        } else {
            self.require_ident()?
        };
        Ok(self.new_expr(ident, start))
    }
}

// General/helpers
impl<'s> Parser<'s> {
    fn ok_if_semicolon(&mut self, stmt: Stmt) -> Result<Stmt> {
        if self.match_take(TokKind::SemiColon).is_none() {
            if let Some(err) = self.misdirection(&stmt) {
                return Err(err);
            }
            Err(MissingSemiColon {
                src: self.src(),
                at: stmt.location().into(),
            })?
        } else {
            Ok(stmt)
        }
    }

    // after a condition parses, a trailing `= 5` or `and b` means the user reached for
    // another language's syntax -- catch it before the body parse swallows the token
    fn condition_misdirection(&mut self, equal_hint: bool) -> Option<shared::Error> {
        let tok = self.peek().ok()?;
        let location = tok.location();
        let (msg, label) = match tok.kind() {
            TokKind::Equal if equal_hint => {
                ("invalid assignment in a condition", "did you mean `==`?")
            }
            TokKind::Ident("and") => ("unknown operator `and`", "mimas spells this `&&`"),
            TokKind::Ident("or") => ("unknown operator `or`", "mimas spells this `||`"),
            _ => return None,
        };
        Some(
            Misdirection {
                src: self.src(),
                at: shared::Location::from(location).into(),
                msg: msg.into(),
                label: label.into(),
            }
            .into(),
        )
    }

    fn misdirection(&mut self, stmt: &Stmt) -> Option<shared::Error> {
        let spellings = |name: &str| match name {
            "and" => Some(("unknown operator `and`", "mimas spells this `&&`")),
            "or" => Some(("unknown operator `or`", "mimas spells this `||`")),
            "elif" => Some(("unknown keyword `elif`", "mimas spells this `else if`")),
            "as" => Some((
                "mimas has no `as` casts",
                "convert with a method instead, e.g. `.to_float()`",
            )),
            _ => None,
        };
        // the culprit is either the next token (mid-statement, e.g. `x as float`) or was
        // already consumed as a lone identifier statement (e.g. the `and` in `if a and b`)
        let tok = self.peek().ok()?;
        let ((msg, label), at) = match tok.kind() {
            TokKind::Ident(name) if spellings(name).is_some() => {
                (spellings(name)?, shared::Location::from(tok.location()))
            }
            _ => {
                let StmtKind::Expr(e) = stmt.kind() else {
                    return None;
                };
                let ExprKind::Ident(id) = e.kind() else {
                    return None;
                };
                (spellings(&id.lexeme)?, stmt.location())
            }
        };
        Some(
            Misdirection {
                src: self.src(),
                at: at.into(),
                msg: msg.into(),
                label: label.into(),
            }
            .into(),
        )
    }

    fn semicolon_check(&mut self, stmt: &Stmt) -> Result<()> {
        let has_semicolon = self.match_take(TokKind::SemiColon).is_some();

        // We'll forgive expression_stmts statements that end with blocks
        //
        // TODO: The check for the block below is commented out because it is not quite sufficient.
        //
        // The following code is valid:
        // ```
        // if a {
        // } else {
        // }
        // {}
        // loop {
        //     break;
        // }
        // ```
        //
        // In other words, these statements don't need semicolons if their type is (). The check
        // below for blocks cheats but checking if there's no yielded value, but that's full of
        // false negatives.
        //
        // For now the compiler simply always treats these as optional, which is more leniant than
        // it should be, as the following code becomes valid:
        //
        // ```
        // if a { 0 } else { 0 }
        // ```
        //
        // This should emit an error that the type of that expression was expected to be (). It
        // needs a semicolon, which would throw out the value of the expression and turn it
        // into a statement. Type analysis is required to properly check this.
        //
        // The control flow expressions that evaluate to ! are correct though!
        //
        // update: I'm pretty sure all this is is that expr stmts must be (), gonna try that
        // ---
        // Expr's that end with blocks do not need semicolons.
        let semicolon_optional = if let StmtKind::Expr(expr) = stmt.kind() {
            matches!(
                expr.kind(),
                ExprKind::If(_)
                    | ExprKind::Match(_)
                    | ExprKind::For(_)
                    | ExprKind::Block(_)
                    | ExprKind::Loop(_)
                    | ExprKind::While(_)
                    | ExprKind::Break(_)
                    | ExprKind::Continue(_)
                    | ExprKind::Collect(_)
                    | ExprKind::Return(_)
                    | ExprKind::Raise(_),
            )
        } else {
            // items handle their own semicolon discipline in `item()` (const/use require, the rest
            // are optional), so by the time they get here it's already resolved.
            matches!(stmt.kind(), StmtKind::Item(_))
        };

        if semicolon_optional || has_semicolon {
            Ok(())
        } else {
            if let Some(err) = self.misdirection(stmt) {
                return Err(err);
            }
            // The semicolon is not optional which means this is an expressions floating in an
            // invalid place.
            Err(MissingSemiColon {
                src: self.src(),
                at: stmt.location().into(),
            })?
        }
    }

    /// Build an `UnexpectedToken` diagnostic from the given lex token. Most parse errors
    /// share this shape, so it lives as a helper here rather than at every call site.
    fn unexpected(&self, tok: Tok<TokKind>) -> UnexpectedToken {
        UnexpectedToken {
            src: self.src(),
            at: shared::Location::from(tok.location()).into(),
            tok: tok.kind.to_string(),
        }
    }

    fn optional_expr(&mut self) -> Option<Expr> {
        // valueless return/break: the next token can't start a value. crucially, do NOT consume
        // the `;` -- it terminates the enclosing statement, not the return/break (otherwise
        // `let x = y else return;` steals the let's own terminator).
        match self.soft_peek().ok().flatten().map(|t| t.kind()) {
            Some(TokKind::SemiColon) | None => None,
            _ => self.expr().ok(),
        }
    }

    fn annotation(&mut self) -> Result<Annotation> {
        self.descend()?;
        let result = self.annotation_inner();
        self.depth -= 1;
        result
    }

    fn annotation_inner(&mut self) -> Result<Annotation> {
        let atom = self.annotation_atom()?;

        if matches!(self.soft_peek()?.map(|t| t.kind()), Some(TokKind::Plus)) {
            let Annotation::Ty(first) = atom else {
                let tok = self.take()?;
                return Err(NonPactInBound {
                    src: self.src(),
                    at: shared::Location::from(tok.location()).into(),
                }
                .into());
            };
            let mut idents = vec![first];
            while self.match_take(TokKind::Plus).is_some() {
                idents.push(self.require_ident()?);
            }
            if let Some(tok) = self
                .match_take(TokKind::Hook)
                .or_else(|| self.match_take(TokKind::Bang))
            {
                return Err(PactBoundConstraint {
                    src: self.src(),
                    at: shared::Location::from(tok.location()).into(),
                }
                .into());
            }
            return Ok(Annotation::Bounds(idents));
        }

        let mut annotation = atom;
        loop {
            if self.match_take(TokKind::Hook).is_some() {
                annotation = Annotation::Option(Box::new(annotation));
            } else if self.match_take(TokKind::Bang).is_some() {
                annotation = Annotation::Result(Box::new(annotation));
            } else if let Some(tok) = self.match_take(TokKind::DoubleHook) {
                // `T??` lexes as one DoubleHook (the coalesce operator), so without this
                // arm it dies on a generic "expected token"
                return Err(DoubledOption {
                    src: self.src(),
                    at: shared::Location::from(tok.location()).into(),
                }
                .into());
            } else {
                break;
            }
        }
        if let Some(tok) = self.match_take(TokKind::Plus) {
            return Err(PactBoundConstraint {
                src: self.src(),
                at: shared::Location::from(tok.location()).into(),
            }
            .into());
        }
        Ok(annotation)
    }

    fn annotation_atom(&mut self) -> Result<Annotation> {
        if matches!(self.peek()?.kind(), TokKind::Ident(_)) {
            let mut segments = vec![self.require_ident()?];
            while self.match_take(TokKind::DoubleColon).is_some() {
                segments.push(self.require_ident()?);
            }
            return Ok(if segments.len() == 1 {
                Annotation::Ty(segments.pop().unwrap())
            } else {
                Annotation::Path(segments)
            });
        }
        let tok = self.take()?;
        match tok.kind() {
            TokKind::TyKw(tykw) => Ok(Annotation::Kw(tykw)),
            TokKind::LeftParenthesis => self.paren_annotation(tok),
            TokKind::LeftSquare => {
                let inner = self.annotation()?;
                self.expect(TokKind::RightSquare)?;
                Ok(Annotation::Array(Box::new(inner)))
            }
            TokKind::TildeLeftBrace => {
                let inner = self.annotation()?;
                self.expect(TokKind::RightBrace)?;
                Ok(Annotation::Dictionary(Box::new(inner)))
            }
            _ => Err(self.unexpected(tok).into()),
        }
    }

    fn paren_annotation(&mut self, open: Tok<TokKind<'s>>) -> Result<Annotation> {
        let mut members = vec![];
        let mut saw_comma = false;
        if self.match_take(TokKind::RightParenthesis).is_none() {
            loop {
                members.push(self.annotation()?);
                let completed = if self.match_take(TokKind::Comma).is_none() {
                    self.expect(TokKind::RightParenthesis)?;
                    true
                } else {
                    saw_comma = true;
                    self.match_take(TokKind::RightParenthesis).is_some()
                };
                if completed {
                    break;
                }
            }
        }

        if self.match_take(TokKind::Arrow).is_some() {
            return Ok(Annotation::Function(members, Box::new(self.annotation()?)));
        }
        if members.is_empty() {
            return Ok(Annotation::Unit);
        }
        if saw_comma {
            return Ok(Annotation::Tuple(members));
        }
        match members.pop().unwrap() {
            bounds @ Annotation::Bounds(_) => Ok(bounds),
            _ => Err(SingleTypeParens {
                src: self.src(),
                at: shared::Location::from(open.location()).into(),
            }
            .into()),
        }
    }
}

// Patterns
impl<'s> Parser<'s> {
    fn pattern(&mut self) -> Result<Pat> {
        self.descend()?;
        let result = self.pattern_inner();
        self.depth -= 1;
        result
    }

    fn pattern_inner(&mut self) -> Result<Pat> {
        let first = self.match_pat_atom()?;
        if self.peek()?.kind() != TokKind::Pipe {
            return Ok(first);
        }
        let start_loc = first.location();
        let mut alts = vec![first];
        while self.match_take(TokKind::Pipe).is_some() {
            alts.push(self.match_pat_atom()?);
        }
        Ok(Pat::new(PatKind::Or(alts), start_loc))
    }

    fn match_pat_atom(&mut self) -> Result<Pat> {
        let start = self.next_tok_boundary();
        let base = self.match_pat_base()?;
        if self.match_take(TokKind::Hook).is_some() {
            return Ok(Pat::new(
                PatKind::NullBind(Box::new(base)),
                self.location(start),
            ));
        }
        Ok(base)
    }

    fn match_pat_base(&mut self) -> Result<Pat> {
        let start = self.next_tok_boundary();
        match self.peek()?.kind() {
            TokKind::LeftParenthesis => {
                self.take()?;
                let mut members = vec![];
                loop {
                    if self.match_take(TokKind::RightParenthesis).is_some() {
                        break;
                    }
                    members.push(self.pattern()?);
                    if self.match_take(TokKind::Comma).is_none() {
                        self.expect(TokKind::RightParenthesis)?;
                        break;
                    }
                }
                Ok(Pat::new(PatKind::Tuple(members), self.location(start)))
            }
            TokKind::Int(_)
            | TokKind::Float(_)
            | TokKind::Hex(_)
            | TokKind::String(_)
            | TokKind::True
            | TokKind::False
            | TokKind::Null => {
                let lit = Literal::try_from(self.peek()?.kind()).unwrap();
                self.take()?;
                Ok(Pat::new(PatKind::Literal(lit), self.location(start)))
            }
            TokKind::Minus => {
                self.take()?;
                let next = self.take()?;
                let lit = match next.kind() {
                    TokKind::Int(i) => Literal::Int(-i),
                    TokKind::Float(f) => Literal::Float(-f),
                    _ => Err(self.unexpected(next))?,
                };
                Ok(Pat::new(PatKind::Literal(lit), self.location(start)))
            }
            TokKind::Ident(_) | TokKind::SelfKeyword => {
                // walk path: ident (:: ident)*
                let first = self.require_ident()?;
                let mut head_expr = self.new_expr(first.clone(), start);
                while self.match_take(TokKind::DoubleColon).is_some() {
                    let right = self.require_member_name()?;
                    head_expr = self.new_expr(
                        Access::DoubleColon {
                            left: head_expr,
                            right,
                        },
                        start,
                    );
                }
                let is_path = matches!(
                    head_expr.kind(),
                    ExprKind::Access(Access::DoubleColon { .. })
                );

                match self.peek()?.kind() {
                    // `Foo(...)` tuple-variant
                    TokKind::LeftParenthesis => {
                        self.take()?;
                        let mut pats = vec![];
                        loop {
                            if self.match_take(TokKind::RightParenthesis).is_some() {
                                break;
                            }
                            pats.push(self.pattern()?);
                            if self.match_take(TokKind::Comma).is_none() {
                                self.expect(TokKind::RightParenthesis)?;
                                break;
                            }
                        }
                        Ok(Pat::new(
                            PatKind::TupleVariant(Box::new(head_expr), pats),
                            self.location(start),
                        ))
                    }
                    // `Foo { ... }` struct
                    TokKind::LeftBrace => {
                        self.take()?;
                        let mut fields = hashbrown::HashMap::new();
                        loop {
                            if self.match_take(TokKind::RightBrace).is_some() {
                                break;
                            }
                            let name = self.require_ident()?;
                            let sub = if self.match_take(TokKind::Equal).is_some() {
                                self.pattern()?
                            } else {
                                // shorthand: `Foo { x }` is `Foo { x = x }`
                                Pat::from(name.clone())
                            };
                            fields.insert(name.lexeme.clone(), sub);
                            if self.match_take(TokKind::Comma).is_none() {
                                self.expect(TokKind::RightBrace)?;
                                break;
                            }
                        }
                        Ok(Pat::new(
                            PatKind::Struct(Box::new(head_expr), fields),
                            self.location(start),
                        ))
                    }
                    // bare identifier -- binding/wildcard
                    _ if !is_path => Ok(Pat::new(PatKind::Ident(first), self.location(start))),
                    // bare path -- `Foo::Bar` with no payload
                    _ => Ok(Pat::new(
                        PatKind::Variant(Box::new(head_expr)),
                        self.location(start),
                    )),
                }
            }
            _ => {
                let tok = self.take()?;
                Err(self.unexpected(tok))?
            }
        }
    }
}

// Lexing tools
impl<'s> Parser<'s> {
    /// Consumes and returns the next tok if it is within the array of types.
    fn match_take_possibilities(&mut self, tok_kinds: &[TokKind<'s>]) -> Option<Tok<TokKind<'s>>> {
        if self.peek().is_ok_and(|tok| tok_kinds.contains(&tok.kind())) {
            Some(self.take().unwrap())
        } else {
            None
        }
    }

    /// Returns the next tok as an Identifier if it is of TokenKind::Identifier.
    fn require_ident(&mut self) -> Result<Ident> {
        let next = self.take()?;
        if let TokKind::Ident(v) = next.kind() {
            Ok(Ident::new(v, next.location().into()))
        } else {
            Err(ExpectedIdentifier {
                src: self.src(),
                at: shared::Location::from(next.location()).into(),
            }
            .into())
        }
    }

    // member-access position (right of `::`): also accepts type keywords as plain names so
    // namespaces like `std::random::int` work without colliding with the type system.
    fn require_member_name(&mut self) -> Result<Ident> {
        let next = self.take()?;
        match next.kind() {
            TokKind::Ident(v) => Ok(Ident::new(v, next.location().into())),
            TokKind::TyKw(kw) => Ok(Ident::new(kw.to_string(), next.location().into())),
            _ => Err(ExpectedIdentifier {
                src: self.src(),
                at: shared::Location::from(next.location()).into(),
            }
            .into()),
        }
    }

    /// Returns the next Token, returning an error if there is none, or if it is
    /// not within the provided array of required types.
    fn require_possibilities(&mut self, toks: &[TokKind<'s>]) -> Result<Tok<TokKind<'s>>> {
        let found_tok = self.take()?;
        if toks.contains(&found_tok.kind()) {
            Ok(found_tok)
        } else {
            Err(ExpectedPossibleTokens {
                src: self.src(),
                at: shared::Location::from(found_tok.location()).into(),
                options: toks
                    .iter()
                    .map(|v| format!("{v:?}"))
                    .collect::<Vec<_>>()
                    .join(", "),
            }
            .into())
        }
    }

    /// Returns the inner field of the next Token if it is an Identifier.
    #[allow(unused)]
    fn match_take_ident(&mut self) -> Result<Option<Ident>> {
        if matches!(self.peek().map(|v| v.kind()), Ok(TokKind::Ident(_))) {
            Ok(Some(self.require_ident()?))
        } else {
            Ok(None)
        }
    }
}

// Token-stream helpers. Used to live on `chompy::parse::Parse`; reimplemented locally so the
// parser can speak `shared::Result` end-to-end instead of bridging chompy diagnostics at every
// hop.
impl<'s> Parser<'s> {
    pub(crate) fn set_cursor(&mut self, target: usize) {
        self.cursor = target;
    }

    /// Start byte of the next token (or the current cursor if we're at EOF).
    pub(crate) fn next_tok_boundary(&mut self) -> usize {
        let cursor = self.cursor;
        self.lexer.peek().map_or(cursor, |tok| {
            tok.as_ref().map_or(cursor, |tok| tok.span().start())
        })
    }

    /// Borrow the next token without advancing. Surfaces lex errors and end-of-input as
    /// diagnostics anchored at the current cursor.
    pub(crate) fn peek(&mut self) -> Result<&Tok<TokKind<'s>>> {
        let next = self.next_tok_boundary();
        let at = self.location(next);
        if self.lexer.peek().is_some_and(|v| v.is_err()) {
            return Err(self.take().unwrap_err());
        }
        // Build the fallback source before reborrowing self.lexer, since the match arms
        // hold the lexer borrow alive across the None path.
        let src = self.src();
        match self.lexer.peek() {
            Some(Ok(tok)) => Ok(tok),
            None => Err(UnexpectedEnd { src, at: at.into() }.into()),
            _ => unreachable!(),
        }
    }

    /// Peek the next token, or `Ok(None)` if input is exhausted. Lex errors still propagate.
    pub(crate) fn soft_peek(&mut self) -> Result<Option<&Tok<TokKind<'s>>>> {
        if self.lexer.peek().is_some_and(|v| v.is_err()) {
            return Err(self.take().unwrap_err());
        }
        match self.lexer.peek() {
            Some(Ok(tok)) => Ok(Some(tok)),
            None => Ok(None),
            _ => unreachable!(),
        }
    }

    /// Consume the next token. End-of-input is an error.
    pub(crate) fn take(&mut self) -> Result<Tok<TokKind<'s>>> {
        let next = self.next_tok_boundary();
        let at = self.location(next);
        match self.lexer.next() {
            Some(Ok(tok)) => {
                self.set_cursor(tok.span().end());
                Ok(tok)
            }
            Some(Err(err)) => Err(err),
            None => Err(UnexpectedEnd {
                src: self.src(),
                at: at.into(),
            }
            .into()),
        }
    }

    /// Consume the next token if it matches `tok_kind`; otherwise leave it unread and return
    /// `None`. Lex errors swallow as `None`; the caller's next `take`/`peek` will surface them.
    pub(crate) fn match_take(&mut self, tok_kind: TokKind<'s>) -> Option<Tok<TokKind<'s>>> {
        match self.peek() {
            Ok(peek) if peek.kind() == tok_kind => Some(self.take().unwrap()),
            _ => None,
        }
    }

    /// A token the grammar *guarantees* is here because we already dispatched on it; a failure
    /// means the parser mis-stepped (our bug, not bad input), so the span marks the offending
    /// token.
    pub(crate) fn take_known(&mut self, known: TokKind<'s>) -> Result<Tok<TokKind<'s>>> {
        let found_tok = self.take()?;
        if found_tok.kind() == known {
            Ok(found_tok)
        } else {
            Err(ParserMisdirection {
                src: self.src(),
                at: shared::Location::from(found_tok.location()).into(),
                expected: known.to_string(),
            }
            .into())
        }
    }

    /// A grammar-mandated token the user must supply next (separator/opener/closer); if it's absent
    /// the span sits at the end of the last consumed token, where it belongs -- not on the stray
    /// one.
    pub(crate) fn expect(&mut self, expected: TokKind<'s>) -> Result<Tok<TokKind<'s>>> {
        let present = self.peek().is_ok_and(|tok| tok.kind() == expected);
        if present {
            self.take()
        } else {
            Err(ExpectedToken {
                src: self.src(),
                at: self.location(self.cursor).into(),
                expected: expected.to_string(),
            }
            .into())
        }
    }
}

/// What `node()` parsed: either a fully-formed stmt, or a bare expression whose role
/// (block-trailing yield vs expression statement) is decided by the caller. Assignments
/// commit to `Stmt` inside `node()` so they never surface as `MaybeYield`.
enum BlockElement {
    Stmt(Stmt),
    MaybeYield(Expr),
}

bitflags! {
    pub(crate) struct Restriction: u32 {
        const NO_STRUCT_LITERAL = 0b0001;
        const REQUIRE_FUNCTION_BODIES = 0b0010;
    }
}

impl Default for Restriction {
    fn default() -> Self {
        Restriction::empty()
    }
}
