use crate::{components::*, errors::*, *};
use chompy::{
    lex::{Lex, Tok, Token},
    utils::Located as _,
};
use hashbrown::HashMap;
use lex::{Lexer, TokKind, interp_end};
use miette::NamedSource;
use shared::{FileId, Located, Location, Span};
use std::{cell::Cell, sync::Arc};

/// Recursively descends mimas source, incrementally returning various
/// statements and expressions.
pub struct Parser<'s> {
    /// Always ends with an `Eof`.
    tokens: Vec<Tok<TokKind<'s>>>,
    /// Each doc comment, keyed by the index of the token after it.
    docs: HashMap<usize, &'s str>,
    /// Index of the next token in `tokens`.
    next: usize,
    /// Spent by looking at the next token and refilled by taking it. Running out means the
    /// parser is stuck.
    fuel: Cell<u32>,
    /// All errors the parser encounterse along its descent.
    errors: Vec<shared::Error>,
    /// The token the last error was reported at. Anything else that goes wrong there is
    /// fallout, and isn't reported.
    last_error: Option<usize>,
    /// The lexer already reported why the input ends where it does.
    cut_short: bool,
    /// The file id of this parser's source.
    file_id: FileId,
    /// The file name of the this parser's source. Todo: do we really need this _and_ the id?
    file_name: String,
    src: NamedSource<Arc<str>>,
    /// Off inside a condition, where a `{` opens the body.
    struct_literals: bool,
    depth: usize,
}

// Basic features
impl<'s> Parser<'s> {
    /// Creates a new parser.
    pub fn new(lexer: Lexer<'s>) -> Self {
        let src = NamedSource::new(lexer.file_name(), Arc::<str>::from(lexer.source()));
        Self::with_src(lexer, src)
    }

    /// Creates a parser whose diagnostics render against `src` rather than the lexer's own
    /// source (the lexer is on a slice of it).
    fn with_src(mut lexer: Lexer<'s>, src: NamedSource<Arc<str>>) -> Self {
        let mut tokens: Vec<_> = lexer.by_ref().collect();
        tokens.push(Tok::new(TokKind::Eof, lexer.end()));

        // Remove the comments from the stream, keep them for later
        let docs = tokens
            .iter()
            .enumerate()
            .filter(|(_, tok)| tok.kind().is_comment())
            .enumerate()
            .filter_map(|(removed, (og_index, tok))| match tok.kind() {
                // once the comments are out, the token after this one is at `og_index - removed`
                TokKind::DocComment(text) => Some((og_index - removed, text)),
                _ => None,
            })
            .collect();
        tokens.retain(|tok| !tok.kind().is_comment());

        let errors = lexer
            .take_errors()
            .into_iter()
            .map(|(diag, location)| {
                let err = LexError {
                    src: src.clone(),
                    diag,
                    at: Location::from(location).into(),
                };
                err.into()
            })
            .collect();
        Self {
            tokens,
            docs,
            next: 0,
            fuel: Cell::new(FUEL),
            errors,
            last_error: None,
            cut_short: lexer.cut_short(),
            file_id: lexer.file_id(),
            file_name: lexer.file_name().into(),
            src,
            struct_literals: true,
            depth: 0,
        }
    }

    /// Parses the whole source into an Ast, along with every error found. Anything that failed
    /// to parse is left in the Ast as [Poison], which later stages treat as unreachable, so only
    /// pass the Ast on when the errors are empty.
    pub fn into_ast(mut self) -> (Ast, Vec<shared::Error>) {
        let mut statements = vec![];
        while !self.at(TokKind::Eof) {
            if self.peek().starts_stmt() {
                statements.push(self.stmt());
            } else {
                self.reject_stmt(self.unexpected_token());
            }
        }
        // the lexer's errors went in first
        self.errors.sort_by_cached_key(|err| {
            let label = err.labels().and_then(|mut labels| labels.next());
            label.map_or(0, |label| label.offset())
        });
        let docs = self
            .docs
            .iter()
            .filter_map(|(&at, doc)| {
                // a doc is for the first name after it, unless its item ends or opens first
                let name = self.tokens[at..]
                    .iter()
                    .take_while(|tok| {
                        !matches!(
                            tok.kind(),
                            TokKind::SemiColon
                                | TokKind::LeftBrace
                                | TokKind::RightBrace
                                | TokKind::Eof
                        )
                    })
                    .find(|tok| tok.kind().is_ident())?;
                let lines: Vec<_> = doc
                    .lines()
                    .map(|line| {
                        let line = line.trim_start().trim_start_matches('/');
                        line.strip_prefix(' ').unwrap_or(line)
                    })
                    .collect();
                Some((name.span().start(), lines.join("\n")))
            })
            .collect();
        (
            Ast::new(self.file_name, self.src, statements, docs),
            self.errors,
        )
    }

    /// The Ast, if the source parsed cleanly. What most callers want -- see [Self::into_ast]
    /// for the errors themselves.
    pub fn try_into_ast(self) -> std::result::Result<Ast, Vec<shared::Error>> {
        let (ast, errors) = self.into_ast();
        if errors.is_empty() {
            Ok(ast)
        } else {
            Err(errors)
        }
    }

    /// Every error found so far. Check this after [Self::expr], which returns poison instead
    /// of failing.
    pub fn errors(&self) -> &[shared::Error] {
        &self.errors
    }

    /// Runs `body` one level deeper into the grammar, or gives up with `None` when that's too
    /// deep (the stack would overflow long before the input ran out). Every cycle in the
    /// grammar has to pass through a call to this.
    fn nested<T>(&mut self, body: impl FnOnce(&mut Self) -> T) -> Option<T> {
        const MAX_DEPTH: usize = 100;
        if self.depth >= MAX_DEPTH {
            self.error(NestingTooDeep {
                src: self.src(),
                at: self.next_location().into(),
            });
            // the rest of the statement would only land us back here
            self.skip();
            self.skip_stmt();
            return None;
        }
        self.depth += 1;
        let parsed = body(self);
        self.depth -= 1;
        Some(parsed)
    }

    /// Runs `body` with struct literals allowed or not, then puts that back as it was. A
    /// condition turns them off, and any brackets inside it turn them back on (nothing in
    /// there can be mistaken for the body).
    fn struct_literals<T>(&mut self, allowed: bool, body: impl FnOnce(&mut Self) -> T) -> T {
        let outer = std::mem::replace(&mut self.struct_literals, allowed);
        let parsed = body(self);
        self.struct_literals = outer;
        parsed
    }

    /// Clone the per-file `NamedSource` for embedding in a diagnostic. Cheap (Arc + String).
    fn src(&self) -> NamedSource<Arc<str>> {
        self.src.clone()
    }

    /// Creates a new expression.
    fn new_expr(&self, expr: impl Into<ExprKind>, start: usize) -> Expr {
        Expr::new(expr.into(), self.location(start))
    }

    /// Creates a new statement.
    fn new_stmt(&self, stmt: impl Into<StmtKind>, start: usize) -> Stmt {
        Stmt::new(stmt.into(), NodeId::new(), self.location(start))
    }

    /// Creates a new pattern.
    fn new_pat(&self, pat: PatKind, start: usize) -> Pat {
        Pat::new(pat, self.location(start))
    }

    /// Creates a poison expression covering whatever was consumed since `start`.
    fn poison_expr(&self, start: usize) -> Expr {
        self.new_expr(Poison, start)
    }

    /// Creates a [Location] from the given position up until our current position (empty at the
    /// cursor if nothing was consumed since `start`).
    fn location(&self, start: usize) -> Location {
        let end = self.cursor();
        Location::new(self.file_id, Span::new(start.min(end), end))
    }
}

// Recursive descent (mimas grammar)
impl<'s> Parser<'s> {
    pub(crate) fn stmt(&mut self) -> Stmt {
        let start = self.next_start();
        match self.node() {
            BlockElement::Stmt(stmt) => stmt,
            BlockElement::MaybeYield(expr) => self.expr_stmt(expr, start),
        }
    }

    /// Routes the next token to the right parse function and reports back what kind of thing came
    /// out: a fully-formed statement, or a bare expression whose role (yielded value vs expression
    /// statement) the caller decides. Assignments finish here as `Stmt`; only true bare exprs
    /// surface as `MaybeYield`.
    fn node(&mut self) -> BlockElement {
        let start = self.next_start();
        match self.peek() {
            TokKind::Let => BlockElement::Stmt(self.let_stmt()),
            TokKind::Module => BlockElement::Stmt(self.module_stmt()),
            kind if kind.starts_item() => {
                let item = self.item();
                self.end_item(&item);
                BlockElement::Stmt(self.new_stmt(item, start))
            }
            _ => {
                let expr = self.expr();

                // TODO: assignment does not belong here... I think
                let Ok(operator) = self.peek().try_into() else {
                    return BlockElement::MaybeYield(expr);
                };
                let left = if matches!(
                    expr.kind(),
                    ExprKind::Access(_) | ExprKind::Ident(_) | ExprKind::Poison(_)
                ) {
                    expr
                } else {
                    self.error(InvalidAssignmentTarget {
                        src: self.src(),
                        at: expr.location().into(),
                    });
                    Expr::new(Poison.into(), expr.location())
                };
                self.advance();
                let assignment = Assignment::new(left, operator, self.expr());
                let stmt = self.new_stmt(assignment, start);
                self.end_stmt(stmt.location());
                BlockElement::Stmt(stmt)
            }
        }
    }

    fn item(&mut self) -> Item {
        fn inner(parser: &mut Parser) -> ItemKind {
            match parser.peek() {
                TokKind::Fn => parser.function().into(),
                TokKind::Struct => parser.struct_decl().into(),
                TokKind::Pact => parser.pact_decl().into(),
                TokKind::Enum => parser.enum_decl().into(),
                TokKind::Impl => parser.impl_decl().into(),
                TokKind::Const => parser.const_decl().into(),
                TokKind::Use => parser.use_decl().map_or(Poison.into(), Into::into),
                _ => {
                    parser.expected("item");
                    Poison.into()
                }
            }
        }
        let start = self.next_start();
        let public = self.eat(TokKind::Pub);
        let kind = self.nested(inner).unwrap_or(Poison.into());
        Item::new(kind, self.location(start), public)
    }

    fn use_decl(&mut self) -> Option<Use> {
        self.bump(TokKind::Use);

        // detailed error, mostly as a humorous nod to rustc
        if self.at(TokKind::Star) {
            self.reject(UseAllModules {
                src: self.src(),
                at: self.next_location().into(),
            });
            return None;
        }

        let mut path = vec![self.require_ident()];
        while self.eat(TokKind::DoubleColon) {
            match self.peek() {
                TokKind::Ident(_) => path.push(self.require_ident()),
                TokKind::Star => {
                    self.bump(TokKind::Star);
                    return Some(Use::All(path));
                }
                TokKind::LeftBrace => {
                    self.bump(TokKind::LeftBrace);
                    let items =
                        self.list(TokKind::RightBrace, TokKind::is_ident, Self::require_ident);
                    return Some(Use::Multi(path, items));
                }
                _ => {
                    self.expected("import");
                    return None;
                }
            }
        }
        // the path ran out, so its last segment is the import
        let item = path.pop()?;
        Some(Use::Singular(path, item))
    }

    fn module_stmt(&mut self) -> Stmt {
        let start = self.next_start();
        self.bump(TokKind::Module);

        // We either are given an ident or an @, which maps to the file name. The lexer's name
        // can be a full path (that's what diagnostics render), so take the stem here.
        let module = if self.eat(TokKind::At) {
            let stem = std::path::Path::new(&self.file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&self.file_name)
                .to_string();
            Module::new(Ident::new(stem, self.location(start)))
        } else {
            // `module a::b;` -- nested path, flattened into one `::`-joined name (the solver
            // splits it back out through ensure_module_path)
            let mut ident = self.require_ident();
            while self.eat(TokKind::DoubleColon) {
                let segment = self.require_ident();
                ident.lexeme = format!("{}::{}", ident.lexeme, segment.lexeme);
            }
            ident.location = self.location(start);
            Module::new(ident)
        };

        let stmt = self.new_stmt(module, start);
        self.end_stmt(stmt.location());
        stmt
    }

    fn let_stmt(&mut self) -> Stmt {
        let start = self.next_start();
        self.bump(TokKind::Let);
        let left = self.pattern();
        let annotation = self.eat(TokKind::Colon).then(|| self.annotation());
        self.expect(TokKind::Equal);
        let right = self.expr();
        let else_branch = self.eat(TokKind::Else).then(|| self.expr());
        let stmt = self.new_stmt(
            Let {
                left,
                annotation,
                right,
                else_branch,
            },
            start,
        );
        self.end_stmt(stmt.location());
        stmt
    }

    fn const_decl(&mut self) -> Const {
        self.bump(TokKind::Const);
        let left = self.require_ident();
        let annotation = self.eat(TokKind::Colon).then(|| self.annotation());
        self.expect(TokKind::Equal);
        let right = self.expr();
        Const {
            left,
            annotation,
            right,
        }
    }

    /// Parses one expression. Anything that fails to parse comes back as [Poison], with its
    /// error in [Self::errors]. See [Self::into_ast] for what that means for the caller.
    pub fn expr(&mut self) -> Expr {
        let expr = self.block_body();
        self.chain_after_block(expr)
    }

    /// The expression dispatch without chaining a trailing block. Control-flow *bodies* parse
    /// through here so postfix after the body (`if c {...} else {...}.foo()`) attaches to the whole
    /// construct, not the inner block; `expr` wraps this with `chain_after_block`.
    fn block_body(&mut self) -> Expr {
        fn inner(parser: &mut Parser) -> Expr {
            match parser.peek() {
                TokKind::Pipe | TokKind::DoublePipe => parser.closure(),
                TokKind::Loop => parser.loop_expr(),
                TokKind::While => parser.while_expr(),
                TokKind::For => parser.for_in(),
                TokKind::If => parser.if_expr(),
                TokKind::Match => parser.match_expr(),
                TokKind::LeftBrace => parser.block(),
                TokKind::Return => parser.return_expr(),
                TokKind::Raise => parser.raise_expr(),
                TokKind::Break => parser.break_stmt(),
                TokKind::Collect => parser.collect(),
                TokKind::Continue => parser.continue_expr(),

                // No keyword expressions found, so start recursive descent
                _ => parser.null_coalecence(),
            }
        }
        let start = self.next_start();
        if !self.peek().starts_expr() {
            self.expected("expression");
            return self.poison_expr(start);
        }
        self.nested(inner)
            .unwrap_or_else(|| self.poison_expr(start))
    }

    fn struct_decl(&mut self) -> Struct {
        self.bump(TokKind::Struct);
        let name = self.require_ident();
        let fields = if self.eat(TokKind::SemiColon) {
            vec![]
        } else if self.eat(TokKind::LeftParenthesis) {
            let mut index = 0;
            self.list(
                TokKind::RightParenthesis,
                |kind| kind == TokKind::Pub || kind.starts_annotation(),
                |p| {
                    let start = p.next_start();
                    let public = p.eat(TokKind::Pub);
                    let annotation = p.annotation();
                    index += 1;
                    StructField {
                        name: FieldKey::Int(index - 1),
                        annotation,
                        public,
                        location: p.location(start),
                    }
                },
            )
        } else if self.expect(TokKind::LeftBrace) {
            self.list(
                TokKind::RightBrace,
                |kind| kind == TokKind::Pub || kind.is_ident(),
                Self::named_field,
            )
        } else {
            vec![]
        };
        Struct { name, fields }
    }

    /// A `name: Type` field, `pub` or not.
    fn named_field(&mut self) -> StructField {
        let start = self.next_start();
        let public = self.eat(TokKind::Pub);
        let name = self.require_ident();
        self.expect(TokKind::Colon);
        let annotation = self.annotation();
        StructField {
            name: FieldKey::Ident(name),
            annotation,
            public,
            location: self.location(start),
        }
    }

    fn pact_decl(&mut self) -> Pact {
        self.bump(TokKind::Pact);
        let pact_name = self.require_ident();
        let mut items = vec![];
        if !self.expect(TokKind::LeftBrace) {
            return Pact::new(pact_name, items);
        }
        while !self.eat(TokKind::RightBrace) {
            let start = self.next_start();
            match self.peek() {
                TokKind::Pub => self.reject(InvalidPubMarker {
                    src: self.src(),
                    at: self.next_location().into(),
                }),
                TokKind::Const => {
                    self.bump(TokKind::Const);
                    let name = self.require_ident();
                    let annotation = if self.eat(TokKind::Colon) {
                        self.annotation()
                    } else {
                        self.error(PactConstAnnotationRequired {
                            src: self.src(),
                            at: self.next_location().into(),
                        });
                        Annotation::Poison(Poison)
                    };
                    self.end_stmt(self.location(start));
                    items.push(PactItem::Const {
                        name,
                        annotation,
                        location: self.location(start),
                    });
                }
                TokKind::Fn => {
                    let (name, parameters, return_type) = self.function_sig(false);
                    // optional default body -- `{ ... }` after the sig. when present, the trailing
                    // `;` is dropped (block-terminated, like normal fns).
                    let default = self.at(TokKind::LeftBrace).then(|| self.block());
                    if default.is_some() {
                        self.eat(TokKind::SemiColon);
                    } else {
                        self.end_stmt(self.location(start));
                    }
                    items.push(PactItem::Fn {
                        name,
                        parameters,
                        return_type,
                        default,
                        location: self.location(start),
                    });
                }
                TokKind::Eof => {
                    self.expect(TokKind::RightBrace);
                    break;
                }
                _ => self.reject_stmt(InvalidPactItem {
                    src: self.src(),
                    at: self.next_location().into(),
                }),
            }
        }
        Pact::new(pact_name, items)
    }

    fn enum_decl(&mut self) -> Enum {
        self.bump(TokKind::Enum);
        let head = self.require_ident();
        let members = if self.expect(TokKind::LeftBrace) {
            self.list(TokKind::RightBrace, TokKind::is_ident, |p| {
                let name = p.require_ident();
                let member = if p.eat(TokKind::LeftParenthesis) {
                    Member::Tuple(p.list(
                        TokKind::RightParenthesis,
                        TokKind::starts_annotation,
                        Self::annotation,
                    ))
                } else if p.eat(TokKind::LeftBrace) {
                    Member::Struct(p.list(TokKind::RightBrace, TokKind::is_ident, |p| {
                        StructField {
                            public: true,
                            ..p.named_field()
                        }
                    }))
                } else {
                    Member::Struct(vec![])
                };
                (name, member)
            })
        } else {
            vec![]
        };
        Enum { head, members }
    }

    fn impl_decl(&mut self) -> Impl {
        self.bump(TokKind::Impl);
        let first = self.require_ident();
        // `impl Foo {}` (inherent) vs `impl Pact for Foo {}` (pact impl). After the first ident,
        // a `for` token disambiguates -- first becomes the pact, second becomes the target.
        let (pact, target) = if self.eat(TokKind::For) {
            let target = self.require_ident();
            (Some(first), target)
        } else {
            (None, first)
        };
        let mut items = vec![];
        if !self.expect(TokKind::LeftBrace) {
            return Impl::new(target, pact, items);
        }
        while !self.eat(TokKind::RightBrace) {
            let kind = if self.at(TokKind::Pub) {
                self.nth(1)
            } else {
                self.peek()
            };
            match kind {
                TokKind::Const | TokKind::Fn => {
                    let item = self.item();
                    self.end_item(&item);
                    items.push(item);
                }
                TokKind::Eof => {
                    self.expect(TokKind::RightBrace);
                    break;
                }
                _ => self.reject_stmt(InvalidImplItem {
                    src: self.src(),
                    at: self.next_location().into(),
                }),
            }
        }
        Impl::new(target, pact, items)
    }

    /// A function up to its body. A pact's signatures don't get `defaults`.
    fn function_sig(&mut self, defaults: bool) -> (Ident, Vec<Binding>, Option<Annotation>) {
        self.bump(TokKind::Fn);
        let name = self.require_ident();
        let parameters = if self.expect(TokKind::LeftParenthesis) {
            let mut first = true;
            self.list(
                TokKind::RightParenthesis,
                |kind| kind == TokKind::SelfKeyword || kind.is_ident(),
                |p| {
                    // only the first parameter can be the receiver
                    let receiver = std::mem::replace(&mut first, false);
                    if receiver && p.at(TokKind::SelfKeyword) {
                        let location = p.next_location();
                        p.bump(TokKind::SelfKeyword);
                        return Binding::new(Ident::new("self", location));
                    }
                    let mut binding = p.binding();
                    binding.right = p.eat(TokKind::Equal).then(|| p.expr());
                    if let Some(default) = binding.right.as_ref().filter(|_| !defaults) {
                        p.error(PactSigDefaultParam {
                            src: p.src(),
                            at: default.location().into(),
                        });
                    }
                    binding
                },
            )
        } else {
            vec![]
        };
        let return_type = self.eat(TokKind::Arrow).then(|| self.annotation());

        (name, parameters, return_type)
    }

    /// A parameter's name, and maybe its type.
    fn binding(&mut self) -> Binding {
        let mut binding = Binding::new(self.require_ident());
        binding.annotation = self.eat(TokKind::Colon).then(|| self.annotation());
        binding
    }

    fn function(&mut self) -> Function {
        let start = self.next_start();
        let (name, parameters, return_type) = self.function_sig(true);
        let body = if self.at(TokKind::LeftBrace) {
            self.block()
        } else {
            self.error_here(MissingFunctionBody {
                src: self.src(),
                at: self.location(start).into(),
            });
            self.poison_expr(self.cursor())
        };

        Function {
            name,
            parameters,
            return_type,
            body,
        }
    }

    fn closure(&mut self) -> Expr {
        let start = self.next_start();
        if self.eat(TokKind::DoublePipe) {
            let return_type = self.eat(TokKind::Arrow).then(|| self.annotation());
            let body = self.expr();
            return self.new_expr(
                Closure {
                    parameters: vec![],
                    body,
                    return_type,
                },
                start,
            );
        }
        self.bump(TokKind::Pipe);
        let parameters = self.list(TokKind::Pipe, TokKind::is_ident, Self::binding);
        let return_type = self.eat(TokKind::Arrow).then(|| self.annotation());
        let body = self.expr();
        self.new_expr(
            Closure {
                parameters,
                body,
                return_type,
            },
            start,
        )
    }

    fn break_stmt(&mut self) -> Expr {
        let start = self.next_start();
        self.bump(TokKind::Break);
        let expr = self.optional_expr();
        self.new_expr(Break::new(expr), start)
    }

    fn collect(&mut self) -> Expr {
        let start = self.next_start();
        self.bump(TokKind::Collect);
        let expr = self.expr();
        self.new_expr(Collect::new(expr), start)
    }

    fn continue_expr(&mut self) -> Expr {
        let start = self.next_start();
        self.bump(TokKind::Continue);
        self.new_expr(ExprKind::Continue(Continue), start)
    }

    fn return_expr(&mut self) -> Expr {
        let start = self.next_start();
        self.bump(TokKind::Return);
        let expr = self.optional_expr();
        self.new_expr(Return::new(expr), start)
    }

    fn raise_expr(&mut self) -> Expr {
        let start = self.next_start();
        self.bump(TokKind::Raise);
        let value = self.expr();
        self.new_expr(Raise::new(value), start)
    }

    fn loop_expr(&mut self) -> Expr {
        let start = self.next_start();
        self.bump(TokKind::Loop);
        let body = self.block_body();
        self.new_expr(Loop::new(body), start)
    }

    fn while_expr(&mut self) -> Expr {
        let start = self.next_start();
        self.bump(TokKind::While);
        let (binding, header) = self.condition();
        let body = self.block_body();
        self.new_expr(
            While {
                header,
                body,
                binding,
            },
            start,
        )
    }

    fn for_in(&mut self) -> Expr {
        let start = self.next_start();
        self.bump(TokKind::For);

        let binding = self.pattern();
        self.expect(TokKind::In);
        let iterator = self.struct_literals(false, |p| {
            let iterator = p.expr();
            let inclusive = p.at(TokKind::DoubleDotEqual);
            if !inclusive && !p.at(TokKind::DoubleDot) {
                return iterator;
            }
            p.advance();
            let start = iterator.span().start();
            let end = p.expr();
            p.new_expr(Range::new(iterator, end, inclusive), start)
        });
        let body = self.block_body();
        self.new_expr(For::new(binding, iterator, body), start)
    }

    fn if_expr(&mut self) -> Expr {
        let start = self.next_start();
        self.bump(TokKind::If);
        let (binding, condition) = self.condition();
        let main_body = self.block_body();
        let else_expr = self.eat(TokKind::Else).then(|| self.block_body());
        self.new_expr(
            If {
                condition,
                main_body,
                else_expr,
                binding,
            },
            start,
        )
    }

    /// What an `if` or a `while` tests, along with the pattern when it's a `let` binding.
    fn condition(&mut self) -> (Option<Pat>, Expr) {
        let binding = self.eat(TokKind::Let).then(|| {
            let binding = self.pattern();
            self.expect(TokKind::Equal);
            binding
        });
        let condition = self.struct_literals(false, Self::expr);

        // a trailing `= 5` or `and b` means the user reached for another language's syntax --
        // catch it before the body parse swallows the token. the rest of the condition gets
        // taken too (so the body still parses) and the whole thing is poison
        let (msg, label) = match self.peek() {
            TokKind::Equal if binding.is_none() => {
                ("invalid assignment in a condition", "did you mean `==`?")
            }
            TokKind::Ident(name @ ("and" | "or")) => {
                foreign_spelling(name).expect("both are in there")
            }
            _ => return (binding, condition),
        };
        self.reject(Misdirection {
            src: self.src(),
            at: self.next_location().into(),
            msg: msg.into(),
            label: label.into(),
        });
        self.struct_literals(false, Self::expr);
        (binding, self.poison_expr(condition.span().start()))
    }

    fn match_expr(&mut self) -> Expr {
        let start = self.next_start();
        self.bump(TokKind::Match);
        let expr = self.struct_literals(false, Self::expr);
        let mut panic_terminator = false;
        let members = if self.expect(TokKind::LeftBrace) {
            self.sequence(
                TokKind::RightBrace,
                |kind| kind == TokKind::Bang || kind.starts_pattern(),
                |p| {
                    if p.eat(TokKind::Bang) {
                        panic_terminator = true;
                        p.eat(TokKind::Comma);
                        // nothing comes after the `!`
                        if !p.at(TokKind::RightBrace) {
                            p.expect(TokKind::RightBrace);
                        }
                        return None;
                    }
                    let pattern = p.pattern();
                    let guard = p.eat(TokKind::If).then(|| p.expr());
                    p.expect(TokKind::FatArrow);
                    let body_is_block = p.at(TokKind::LeftBrace);
                    let body = p.expr();
                    // block-bodied arms can omit the trailing comma
                    if !p.eat(TokKind::Comma) && !body_is_block && !p.at(TokKind::RightBrace) {
                        p.expect(TokKind::Comma);
                    }
                    Some(MatchCase::new(pattern, guard, body))
                },
            )
        } else {
            vec![]
        };
        let members = members.into_iter().flatten().collect();
        self.new_expr(Match::new(expr, members, panic_terminator), start)
    }

    fn null_coalecence(&mut self) -> Expr {
        let start = self.next_start();
        let expr = self.binary(1);
        if self.eat(TokKind::DoubleHook) {
            let value = self.expr();
            self.new_expr(Coalescence::new(expr, value), start)
        } else if self.eat(TokKind::Absolve) {
            let handler = self.expr();
            self.new_expr(Absolve::new(expr, handler), start)
        } else {
            expr
        }
    }

    /// Precedence climbing over the binary operators. `min_power` is the loosest operator this
    /// call may take; anything looser belongs to the caller.
    fn binary(&mut self, min_power: u8) -> Expr {
        let start = self.next_start();
        let mut left = self.unary();
        // only looser operators can follow an `in`, which doesn't chain
        let mut max_power = u8::MAX;
        while let Some((op, power)) = BinaryOp::of(self.peek())
            && (min_power..max_power).contains(&power)
        {
            self.advance();
            let right = self.binary(power + 1);
            if matches!(op, BinaryOp::In(_)) {
                max_power = power;
            }
            left = match op {
                BinaryOp::Logical(op) => self.new_expr(Logical::new(left, op, right), start),
                BinaryOp::Equality(op) => self.new_expr(Equality::new(left, op, right), start),
                BinaryOp::In(condition) => self.new_expr(In::new(left, right, condition), start),
                BinaryOp::Eval(op) => self.new_expr(Evaluation::new(left, op, right), start),
            };
        }
        left
    }

    fn unary(&mut self) -> Expr {
        let start = self.next_start();
        let Ok(operator) = self.peek().try_into() else {
            return self.fstring();
        };
        self.advance();
        // `----...` recurses here without passing back through `expr`
        let right = self
            .nested(Self::unary)
            .unwrap_or_else(|| self.poison_expr(self.cursor()));
        self.new_expr(Unary::new(operator, right), start)
    }

    fn fstring(&mut self) -> Expr {
        /// Parses the expression inside a `{...}`. It gets a parser of its own, on the
        /// interpolation's slice of the real source.
        fn interpolation<'s>(outer: &mut Parser<'s>, body: &'s str, offset: usize) -> Expr {
            let lexer = Lexer::new(body, outer.file_id, outer.file_name.clone());
            let mut parser = Parser::with_src(lexer.with_offset(offset), outer.src());
            // nested f-strings would otherwise restart the depth guard
            parser.depth = outer.depth;
            let expr = parser.expr();
            if !parser.at(TokKind::Eof) {
                parser.error(parser.unexpected_token());
            }
            outer.errors.extend(parser.errors);
            expr
        }
        // `{` and `}` join the quote-chars so chompy's unescape resolves `\{` -> `{` and
        // `\}` -> `}` (the brace-escape form, sibling to `{{`/`}}`).
        fn unescape(literal: &str) -> FStringPart {
            FStringPart::Literal(chompy::utils::unescape(literal, &['\\'], &['"', '{', '}']))
        }

        let start = self.next_start();
        let TokKind::FString(content) = self.peek() else {
            return self.literal();
        };
        self.advance();
        // the token covers `f"..."`, so the content begins two bytes in
        let content_start = start + 2;

        let mut parts = vec![];
        let mut literal = String::new();
        let mut at = 0;
        while let Some(c) = content[at..].chars().next() {
            at += c.len_utf8();
            let rest = &content[at..];
            match c {
                '\\' => {
                    literal.push(c);
                    if let Some(escaped) = rest.chars().next() {
                        literal.push(escaped);
                        at += escaped.len_utf8();
                    }
                }
                // `{{` and `}}` are a literal `{` and `}`
                '{' | '}' if rest.starts_with(c) => {
                    literal.push(c);
                    at += 1;
                }
                '{' => {
                    if !literal.is_empty() {
                        parts.push(unescape(&literal));
                        literal.clear();
                    }
                    let Some(end) = interp_end(rest) else {
                        let open = content_start + at - 1;
                        self.error(UnterminatedFStringExpr {
                            src: self.src(),
                            at: Location::new(self.file_id, Span::new(open, open + 1)).into(),
                        });
                        break;
                    };
                    let expr = interpolation(self, &rest[..end], content_start + at);
                    parts.push(FStringPart::Expr(expr));
                    at += end + 1;
                }
                _ => literal.push(c),
            }
        }
        if !literal.is_empty() {
            parts.push(unescape(&literal));
        }

        let expr = self.new_expr(FString::new(parts), start);
        self.chain_accesses(expr)
    }

    fn literal(&mut self) -> Expr {
        let start = self.next_start();
        if let Ok(literal) = Literal::try_from(self.peek()) {
            self.advance();
            let expr = self.new_expr(literal, start);

            // todo: this might allow "hello"() or true[]" etc. only dot accesses are okay on lits"
            self.chain_accesses(expr)
        } else if self.eat(TokKind::LeftSquare) {
            let elements = self.list(TokKind::RightSquare, TokKind::starts_expr, |p| {
                p.struct_literals(true, Self::expr)
            });
            let expr = self.new_expr(Literal::Array(elements), start);
            self.chain_accesses(expr)
        } else if self.eat(TokKind::TildeLeftBrace) {
            let elements = self.list(TokKind::RightBrace, TokKind::is_ident, |p| {
                let name = p.require_ident();
                p.expect(TokKind::Equal);
                (name, p.struct_literals(true, Self::expr))
            });
            let expr = self.new_expr(Literal::Dictionary(elements), start);
            self.chain_accesses(expr)
        } else {
            let name = self.supreme();
            if !self.struct_literals || !self.eat(TokKind::LeftBrace) {
                return name;
            }
            let fields = self.list(TokKind::RightBrace, TokKind::is_ident, |p| {
                let field = p.require_ident();
                // shorthand: `Foo { x }` is `Foo { x = x }`
                let value = if p.at(TokKind::Comma) || p.at(TokKind::RightBrace) {
                    p.new_expr(field.clone(), field.location.span().start())
                } else {
                    p.expect(TokKind::Equal);
                    p.expr()
                };
                (FieldKey::Ident(field), value)
            });
            let expr = self.new_expr(Literal::Struct(StructLiteral { name, fields }), start);
            self.chain_accesses(expr)
        }
    }

    fn supreme(&mut self) -> Expr {
        let expr = self.primary();
        self.chain_accesses(expr)
    }

    /// An expression with no postfix on it yet. [Self::chain_accesses] adds those.
    fn primary(&mut self) -> Expr {
        let start = self.next_start();
        // `int::random` etc. -- synthesize an Ident at the expr head so the regular library map
        // lookup handles it. TyKw is only legal here in `TyKw ::` shape.
        if let TokKind::TyKw(ty) = self.peek() {
            self.advance();
            if !self.at(TokKind::DoubleColon) {
                self.expect(TokKind::DoubleColon);
            }
            return self.new_expr(Ident::new(ty.to_string(), self.location(start)), start);
        }
        self.parentheticals()
    }

    /// Chains any trailing access expressions (`.field`, `[key]`, `::member`,
    /// `()` calls, `!` unwraps, bare `?` postfix) onto an already-parsed expression.
    fn chain_accesses(&mut self, expr: Expr) -> Expr {
        // postfix on a failed expression only ever earns a second error for the same mistake
        if matches!(expr.kind(), ExprKind::Poison(_)) {
            return expr;
        }
        let mut expr = expr;
        loop {
            expr = match self.peek() {
                TokKind::LeftParenthesis => self.call(expr),
                TokKind::LeftSquare | TokKind::HookLeftSquare => self.square_access(expr),
                TokKind::DoubleColon => self.colon_access(expr),
                TokKind::Dot | TokKind::HookDot => self.dot_access(expr),
                TokKind::Bang => self.unwrap(expr),
                // bare trailing ?'s do nothing but are permitted. future warning
                TokKind::Hook => {
                    self.bump(TokKind::Hook);
                    expr
                }
                _ => break expr,
            }
        }
    }

    /// Postfix chaining for block expressions. `(` and `[` directly after a block are ambiguous
    /// with a following statement (`if c {}` newline `[x]`), so they need parens; `.`/`::`/`!`/`?`
    /// can't start a statement, so once one attaches we hand off to the full `chain_accesses`.
    fn chain_after_block(&mut self, expr: Expr) -> Expr {
        match self.peek() {
            TokKind::Dot
            | TokKind::HookDot
            | TokKind::DoubleColon
            | TokKind::Bang
            | TokKind::Hook => self.chain_accesses(expr),
            _ => expr,
        }
    }

    fn unwrap(&mut self, left: Expr) -> Expr {
        let start = left.span().start();
        self.bump(TokKind::Bang);
        self.new_expr(Unwrap { expr: left }, start)
    }

    fn call(&mut self, left: Expr) -> Expr {
        let start = left.span().start();
        self.bump(TokKind::LeftParenthesis);
        let mut named = false;
        let arguments = self.list(TokKind::RightParenthesis, TokKind::starts_expr, |p| {
            let value = p.struct_literals(true, Self::expr);
            if !p.at(TokKind::Equal) {
                if named {
                    p.error(NamedBeforePositional {
                        src: p.src(),
                        at: value.location().into(),
                    });
                }
                return Argument { name: None, value };
            }
            named = true;
            let name = match value.kind() {
                ExprKind::Ident(ident) => Some(ident.clone()),
                _ => {
                    p.error(p.unexpected_token());
                    None
                }
            };
            p.bump(TokKind::Equal);
            let value = p.expr();
            Argument { name, value }
        });
        self.new_expr(Call::new(left, arguments), start)
    }

    fn dot_access(&mut self, left: Expr) -> Expr {
        let start = left.span().start();
        let kind = AccessKind::try_from(self.advance().kind()).expect("dispatched on a dot");
        // the member spans just its own token, not back to the start of the receiver
        let member_start = self.next_start();
        let right = match self.peek() {
            TokKind::Ident(_) => {
                let ident = self.require_ident();
                self.new_expr(ident, member_start)
            }
            // take the int token directly -- `parser.literal()` would chain further accesses,
            // which would steal a trailing `.foo()` from the *outer* dot (`a.0.pairs()` would
            // misparse as `a . (0.pairs())`). The outer chain_accesses loop owns chaining.
            TokKind::Int(index) => {
                self.advance();
                self.new_expr(Literal::Int(index), member_start)
            }
            _ => {
                self.error_here(InvalidDotAccess {
                    src: self.src(),
                    at: self.location(start).into(),
                });
                self.poison_expr(start)
            }
        };
        self.new_expr(Access::Dot { left, right, kind }, start)
    }

    fn colon_access(&mut self, left: Expr) -> Expr {
        let start = left.span().start();
        self.bump(TokKind::DoubleColon);
        let right = self.require_member_name();
        self.new_expr(Access::DoubleColon { left, right }, start)
    }

    fn square_access(&mut self, left: Expr) -> Expr {
        let start = left.span().start();
        let kind = AccessKind::try_from(self.advance().kind()).expect("dispatched on a square");
        let key = self.struct_literals(true, Self::expr);
        self.expect(TokKind::RightSquare);
        self.new_expr(Access::Square { left, key, kind }, start)
    }

    fn parentheticals(&mut self) -> Expr {
        let start = self.next_start();
        if !self.eat(TokKind::LeftParenthesis) {
            return self.block();
        }
        if self.eat(TokKind::RightParenthesis) {
            return self.new_expr(Literal::Unit, start);
        }
        let first = self.struct_literals(true, Self::expr);
        if !self.eat(TokKind::Comma) {
            self.expect(TokKind::RightParenthesis);
            return self.new_expr(Grouping::new(first), start);
        }
        let mut members = vec![first];
        members.extend(
            self.list(TokKind::RightParenthesis, TokKind::starts_expr, |p| {
                p.struct_literals(true, Self::expr)
            }),
        );
        self.new_expr(Literal::Tuple(members), start)
    }

    fn block(&mut self) -> Expr {
        let start = self.next_start();
        if !self.eat(TokKind::LeftBrace) {
            return self.ident();
        }
        let mut body: Vec<Stmt> = vec![];
        let yielded_expr = loop {
            if self.eat(TokKind::RightBrace) {
                break None;
            }
            if self.at(TokKind::Eof) {
                self.expect(TokKind::RightBrace);
                break None;
            }
            if !self.peek().starts_stmt() {
                self.reject_stmt(self.unexpected_token());
                continue;
            }
            let node_start = self.next_start();
            match self.node() {
                BlockElement::Stmt(stmt) => body.push(stmt),
                // last node in the block -- this is the yield, not a new stmt
                BlockElement::MaybeYield(expr) if self.eat(TokKind::RightBrace) => {
                    break Some(expr);
                }
                BlockElement::MaybeYield(expr) => body.push(self.expr_stmt(expr, node_start)),
            }
        };
        self.new_expr(Block { body, yielded_expr }, start)
    }

    /// The bottom of the descent. Anything that isn't an identifier by now isn't an expression.
    fn ident(&mut self) -> Expr {
        let start = self.next_start();
        match self.peek() {
            // we're just gonna hijack this guy...
            TokKind::SelfKeyword => {
                let lexeme = self.advance().to_string();
                self.new_expr(Ident::new(lexeme, self.location(start)), start)
            }
            TokKind::Ident(_) => {
                let ident = self.require_ident();
                self.new_expr(ident, start)
            }
            _ => {
                self.expected("expression");
                self.poison_expr(start)
            }
        }
    }
}

// General/helpers
impl<'s> Parser<'s> {
    /// Turns a bare expression into a statement, which has to end like one.
    fn expr_stmt(&mut self, expr: Expr, start: usize) -> Stmt {
        // Expr's that end with blocks do not need semicolons.
        let semicolon_optional = matches!(
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
        );

        // a keyword from another language (the `elif` in `if a {} elif b {}`) parses as a
        // statement of its own, so the next token is no help in spotting it
        if let ExprKind::Ident(ident) = expr.kind()
            && let Some((msg, label)) = foreign_spelling(&ident.lexeme)
            && !self.at(TokKind::SemiColon)
        {
            self.error(Misdirection {
                src: self.src(),
                at: ident.location.into(),
                msg: msg.into(),
                label: label.into(),
            });
        }

        let stmt = self.new_stmt(StmtKind::Expr(expr), start);
        if semicolon_optional {
            self.eat(TokKind::SemiColon);
        } else {
            self.end_stmt(stmt.location());
        }
        stmt
    }

    /// Ends an item the way its kind does: a `const` or a `use` needs its `;`, and the rest
    /// may have one.
    fn end_item(&mut self, item: &Item) {
        if matches!(item.kind(), ItemKind::Const(_) | ItemKind::Use(_)) {
            self.end_stmt(item.location());
        } else {
            self.eat(TokKind::SemiColon);
        }
    }

    /// Ends a statement on its `;`. When something the statement couldn't use is in the way, the
    /// rest of the line goes with it.
    fn end_stmt(&mut self, stmt: Location) {
        if self.eat(TokKind::SemiColon) {
            return;
        }
        let stray = !self.peek().ends_stmt() && !self.at_line_start();
        if let TokKind::Ident(name) = self.peek()
            && let Some((msg, label)) = foreign_spelling(name)
        {
            self.error(Misdirection {
                src: self.src(),
                at: self.next_location().into(),
                msg: msg.into(),
                label: label.into(),
            });
        } else if stray {
            self.error(self.unexpected_token());
        } else {
            self.error(MissingSemiColon {
                src: self.src(),
                at: stmt.into(),
            });
        }
        if stray {
            self.skip_stmt();
            self.eat(TokKind::SemiColon);
        }
    }

    /// The value of a `return` or a `break`, when there is one.
    fn optional_expr(&mut self) -> Option<Expr> {
        self.peek().starts_expr().then(|| self.expr())
    }

    fn annotation(&mut self) -> Annotation {
        fn inner(parser: &mut Parser) -> Annotation {
            let atom = parser.annotation_atom();

            if parser.at(TokKind::Plus) {
                let Annotation::Ty(first) = atom else {
                    parser.reject(NonPactInBound {
                        src: parser.src(),
                        at: parser.next_location().into(),
                    });
                    return Annotation::Poison(Poison);
                };
                let mut idents = vec![first];
                while parser.eat(TokKind::Plus) {
                    idents.push(parser.require_ident());
                }
                if parser.at(TokKind::Hook) || parser.at(TokKind::Bang) {
                    parser.reject(PactBoundConstraint {
                        src: parser.src(),
                        at: parser.next_location().into(),
                    });
                }
                return Annotation::Bounds(idents);
            }

            let mut annotation = atom;
            loop {
                if parser.eat(TokKind::Hook) {
                    annotation = Annotation::Option(Box::new(annotation));
                } else if parser.eat(TokKind::Bang) {
                    annotation = Annotation::Result(Box::new(annotation));
                } else if parser.at(TokKind::DoubleHook) {
                    // `T??` lexes as one DoubleHook (the coalesce operator), so without this
                    // arm it dies on a generic "expected token"
                    parser.error(DoubledOption {
                        src: parser.src(),
                        at: parser.next_location().into(),
                    });
                    parser.bump(TokKind::DoubleHook);
                    annotation = Annotation::Option(Box::new(annotation));
                } else {
                    break;
                }
            }
            if parser.at(TokKind::Plus) {
                parser.reject(PactBoundConstraint {
                    src: parser.src(),
                    at: parser.next_location().into(),
                });
            }
            annotation
        }
        if !self.peek().starts_annotation() {
            self.expected("type");
            return Annotation::Poison(Poison);
        }
        self.nested(inner).unwrap_or(Annotation::Poison(Poison))
    }

    fn annotation_atom(&mut self) -> Annotation {
        match self.peek() {
            TokKind::Ident(_) => {
                let mut segments = vec![self.require_ident()];
                while self.eat(TokKind::DoubleColon) {
                    segments.push(self.require_ident());
                }
                match segments.len() {
                    1 => Annotation::Ty(segments.remove(0)),
                    _ => Annotation::Path(segments),
                }
            }
            TokKind::TyKw(tykw) => {
                self.advance();
                Annotation::Kw(tykw)
            }
            TokKind::LeftParenthesis => self.paren_annotation(),
            TokKind::LeftSquare => {
                self.bump(TokKind::LeftSquare);
                let inner = self.annotation();
                self.expect(TokKind::RightSquare);
                Annotation::Array(Box::new(inner))
            }
            TokKind::TildeLeftBrace => {
                self.bump(TokKind::TildeLeftBrace);
                let inner = self.annotation();
                self.expect(TokKind::RightBrace);
                Annotation::Dictionary(Box::new(inner))
            }
            _ => {
                self.expected("type");
                Annotation::Poison(Poison)
            }
        }
    }

    /// Everything that opens with a `(`: unit, a tuple, a function, or a parenthesized bound.
    fn paren_annotation(&mut self) -> Annotation {
        let open = self.next_location();
        self.bump(TokKind::LeftParenthesis);
        let mut members = vec![];
        let mut tuple = false;
        if !self.eat(TokKind::RightParenthesis) {
            members.push(self.annotation());
            tuple = self.eat(TokKind::Comma);
            if tuple {
                members.extend(self.list(
                    TokKind::RightParenthesis,
                    TokKind::starts_annotation,
                    Self::annotation,
                ));
            } else {
                self.expect(TokKind::RightParenthesis);
            }
        }

        if self.eat(TokKind::Arrow) {
            return Annotation::Function(members, Box::new(self.annotation()));
        }
        if tuple {
            return Annotation::Tuple(members);
        }
        match members.pop() {
            None => Annotation::Unit,
            // no use complaining about the parens when what's inside them didn't parse
            Some(inner @ (Annotation::Bounds(_) | Annotation::Poison(_))) => inner,
            Some(single) => {
                self.error(SingleTypeParens {
                    src: self.src(),
                    at: open.into(),
                });
                single
            }
        }
    }

    /// Parses `element`s up to `closer` (whose opener is already taken), with a `,` between
    /// them and maybe one after the last.
    fn list<T>(
        &mut self,
        closer: TokKind<'s>,
        starts: impl Fn(TokKind<'s>) -> bool,
        mut element: impl FnMut(&mut Self) -> T,
    ) -> Vec<T> {
        self.sequence(closer, &starts, |p| {
            let parsed = element(p);
            // a missing `,` right before another element is reported, and the list carries on
            if !p.eat(TokKind::Comma) && starts(p.peek()) {
                p.expect(TokKind::Comma);
            }
            parsed
        })
    }

    /// Parses `element`s up to `closer`. An element begins on a token `starts` accepts, and has
    /// to take it. Anything else is reported and skipped through the next `,`. A token that
    /// something further out is waiting for ends the sequence early.
    fn sequence<T>(
        &mut self,
        closer: TokKind<'s>,
        starts: impl Fn(TokKind<'s>) -> bool,
        mut element: impl FnMut(&mut Self) -> T,
    ) -> Vec<T> {
        let mut elements = vec![];
        while !self.eat(closer) {
            let kind = self.peek();
            if starts(kind) {
                elements.push(element(self));
            } else if kind.ends_list() {
                self.expect(closer);
                break;
            } else {
                self.error(self.unexpected_token());
                while !self.at(TokKind::Comma) && !self.peek().is_boundary() {
                    self.skip();
                }
                self.eat(TokKind::Comma);
            }
        }
        elements
    }

    /// Reports that `what` was expected where the next token is. The token is skipped, unless
    /// something further out is waiting for it.
    fn expected(&mut self, what: &'static str) {
        self.error_here(Expected {
            src: self.src(),
            at: self.next_location().into(),
            what,
        });
        if !self.peek().is_anchor() {
            self.skip();
        }
    }

    /// Reports the next token with `err`, then skips it and the rest of its statement.
    fn reject_stmt(&mut self, err: impl Into<shared::Error>) {
        self.reject(err);
        self.skip_stmt();
        self.eat(TokKind::SemiColon);
    }

    /// Reports the next token with `err`, then skips it.
    fn reject(&mut self, err: impl Into<shared::Error>) {
        self.error_here(err);
        self.skip();
    }

    /// Skips what's left of a statement that stopped making sense, up to its `;` or the end of
    /// its line.
    fn skip_stmt(&mut self) {
        while !self.peek().ends_stmt() && !self.at_line_start() {
            self.skip();
        }
    }

    /// Skips the next token, along with everything up to its matching closer if it opens a
    /// group. A closer of the wrong kind stops the skip (it belongs to something further out).
    /// Whatever goes wrong where the skip ends is fallout.
    fn skip(&mut self) {
        let mut open: Vec<TokKind<'s>> = vec![];
        loop {
            let kind = self.peek();
            let mismatched = kind.is_closer() && open.last().is_some_and(|open| *open != kind);
            if kind == TokKind::Eof || mismatched {
                break;
            }
            self.advance();
            match kind {
                TokKind::LeftParenthesis => open.push(TokKind::RightParenthesis),
                TokKind::LeftSquare | TokKind::HookLeftSquare => open.push(TokKind::RightSquare),
                TokKind::LeftBrace | TokKind::TildeLeftBrace => open.push(TokKind::RightBrace),
                kind if kind.is_closer() => {
                    open.pop();
                }
                _ => {}
            }
            if open.is_empty() {
                break;
            }
        }
        self.last_error = Some(self.next);
    }

    /// Creates the error for a token that doesn't belong where it is.
    fn unexpected_token(&self) -> UnexpectedToken {
        UnexpectedToken {
            src: self.src(),
            at: self.next_location().into(),
            tok: self.peek().to_string(),
        }
    }

    /// Records an error about the next token, or the end of input if that's where we are.
    fn error_here(&mut self, err: impl Into<shared::Error>) {
        if self.at(TokKind::Eof) {
            self.error(UnexpectedEnd {
                src: self.src(),
                at: self.location(self.cursor()).into(),
            });
        } else {
            self.error(err);
        }
    }

    /// Records an error, unless one was already reported at this token.
    fn error(&mut self, err: impl Into<shared::Error>) {
        let explained = self.cut_short && self.at(TokKind::Eof);
        if self.last_error != Some(self.next) && !explained {
            self.errors.push(err.into());
        }
        self.last_error = Some(self.next);
    }
}

// Patterns
impl<'s> Parser<'s> {
    fn pattern(&mut self) -> Pat {
        fn inner(parser: &mut Parser) -> Pat {
            let first = parser.match_pat_atom();
            if !parser.at(TokKind::Pipe) {
                return first;
            }
            let location = first.location();
            let mut alts = vec![first];
            while parser.eat(TokKind::Pipe) {
                alts.push(parser.match_pat_atom());
            }
            Pat::new(PatKind::Or(alts), location)
        }
        let start = self.next_start();
        if !self.peek().starts_pattern() {
            self.expected("pattern");
            return self.new_pat(PatKind::Poison(Poison), start);
        }
        self.nested(inner)
            .unwrap_or_else(|| self.new_pat(PatKind::Poison(Poison), start))
    }

    fn match_pat_atom(&mut self) -> Pat {
        let start = self.next_start();
        let base = self.match_pat_base();
        if self.eat(TokKind::Hook) {
            return self.new_pat(PatKind::NullBind(Box::new(base)), start);
        }
        base
    }

    fn match_pat_base(&mut self) -> Pat {
        let start = self.next_start();
        let kind = self.peek();
        match kind {
            TokKind::LeftParenthesis => {
                self.bump(TokKind::LeftParenthesis);
                let members = self.list(
                    TokKind::RightParenthesis,
                    TokKind::starts_pattern,
                    Self::pattern,
                );
                self.new_pat(PatKind::Tuple(members), start)
            }
            TokKind::Minus => {
                self.bump(TokKind::Minus);
                let literal = match self.peek() {
                    TokKind::Int(i) => Literal::Int(-i),
                    TokKind::Float(f) => Literal::Float(-f),
                    _ => {
                        self.expected("number");
                        return self.new_pat(PatKind::Poison(Poison), start);
                    }
                };
                self.advance();
                self.new_pat(PatKind::Literal(literal), start)
            }
            TokKind::Ident(_) => {
                // walk path: ident (:: ident)*
                let first = self.require_ident();
                let mut head_expr = self.new_expr(first.clone(), start);
                let mut is_path = false;
                while self.eat(TokKind::DoubleColon) {
                    let right = self.require_member_name();
                    let left = head_expr;
                    head_expr = self.new_expr(Access::DoubleColon { left, right }, start);
                    is_path = true;
                }

                match self.peek() {
                    // `Foo(...)` tuple-variant
                    TokKind::LeftParenthesis => {
                        self.bump(TokKind::LeftParenthesis);
                        let pats = self.list(
                            TokKind::RightParenthesis,
                            TokKind::starts_pattern,
                            Self::pattern,
                        );
                        self.new_pat(PatKind::TupleVariant(Box::new(head_expr), pats), start)
                    }
                    // `Foo { ... }` struct
                    TokKind::LeftBrace => {
                        self.bump(TokKind::LeftBrace);
                        let fields = self.list(TokKind::RightBrace, TokKind::is_ident, |p| {
                            let name = p.require_ident();
                            let sub = if p.eat(TokKind::Equal) {
                                p.pattern()
                            } else {
                                // shorthand: `Foo { x }` is `Foo { x = x }`
                                Pat::from(name.clone())
                            };
                            (name.lexeme, sub)
                        });
                        let fields = fields.into_iter().collect();
                        self.new_pat(PatKind::Struct(Box::new(head_expr), fields), start)
                    }
                    // bare path -- `Foo::Bar` with no payload
                    _ if is_path => self.new_pat(PatKind::Variant(Box::new(head_expr)), start),
                    // bare identifier -- binding/wildcard
                    _ => self.new_pat(PatKind::Ident(first), start),
                }
            }
            _ => match Literal::try_from(kind) {
                Ok(literal) => {
                    self.advance();
                    self.new_pat(PatKind::Literal(literal), start)
                }
                Err(()) => {
                    self.expected("pattern");
                    self.new_pat(PatKind::Poison(Poison), start)
                }
            },
        }
    }
}

// Lexing tools
impl<'s> Parser<'s> {
    /// Returns the next tok as an Identifier, or poison (after reporting) if it isn't one.
    fn require_ident(&mut self) -> Ident {
        let location = self.next_location();
        if let TokKind::Ident(lexeme) = self.peek() {
            self.advance();
            return Ident::new(lexeme, location);
        }
        self.expected("identifier");
        // `Ident` is a plain struct with nowhere to put a variant, so its poison is a lexeme no
        // real identifier can have
        Ident::new(POISON, self.location(self.next_start()))
    }

    // member-access position (right of `::`): also accepts type keywords as plain names so
    // namespaces like `std::random::int` work without colliding with the type system.
    fn require_member_name(&mut self) -> Ident {
        let location = self.next_location();
        if let TokKind::TyKw(kw) = self.peek() {
            self.advance();
            return Ident::new(kw.to_string(), location);
        }
        self.require_ident()
    }

    /// A grammar-mandated token the user must supply next (separator/opener/closer). If it's
    /// absent the span sits at the end of the last consumed token, where it belongs -- not on the
    /// stray one. Nothing is consumed when it's missing.
    fn expect(&mut self, kind: TokKind<'s>) -> bool {
        let found = self.eat(kind);
        if !found {
            self.error_here(ExpectedToken {
                src: self.src(),
                at: self.location(self.cursor()).into(),
                expected: kind.to_string(),
            });
        }
        found
    }

    /// Takes the next token if it's a `kind`, and returns whether it was.
    fn eat(&mut self, kind: TokKind<'s>) -> bool {
        let found = self.at(kind);
        if found {
            self.advance();
        }
        found
    }

    /// Takes a token the grammar already dispatched on.
    fn bump(&mut self, kind: TokKind<'s>) {
        debug_assert!(self.at(kind), "dispatched on {kind}");
        self.advance();
    }

    /// Takes the next token. The `Eof` stays put.
    fn advance(&mut self) -> Tok<TokKind<'s>> {
        let tok = self.tokens[self.next];
        if tok.kind() != TokKind::Eof {
            self.next += 1;
            self.fuel.set(FUEL);
        }
        tok
    }

    /// Returns if the next token is a `kind`.
    fn at(&self, kind: TokKind<'s>) -> bool {
        self.peek() == kind
    }

    /// Kind of the next token.
    fn peek(&self) -> TokKind<'s> {
        self.nth(0)
    }

    /// Kind of the token `n` past the next one.
    fn nth(&self, n: usize) -> TokKind<'s> {
        let fuel = self.fuel.get();
        assert!(
            fuel > 0,
            "the parser is stuck at byte {}",
            self.next_start()
        );
        self.fuel.set(fuel - 1);
        let last = self.tokens.len() - 1;
        self.tokens[(self.next + n).min(last)].kind()
    }

    /// Location of the next token.
    fn next_location(&self) -> Location {
        self.tokens[self.next].location().into()
    }

    /// Start byte of the next token.
    fn next_start(&self) -> usize {
        self.tokens[self.next].span().start()
    }

    /// Whether the next token is the first on its line.
    fn at_line_start(&self) -> bool {
        self.src.inner()[self.cursor()..self.next_start()].contains('\n')
    }

    /// End byte of the last token taken.
    fn cursor(&self) -> usize {
        match self.next.checked_sub(1) {
            Some(last) => self.tokens[last].span().end(),
            None => self.next_start(),
        }
    }
}

/// How many times the parser can look at a token before it has to take it. Nearly everything
/// stays in the dozens. The exception is backing out of the depth guard, where each of the 100
/// levels looks at whatever stopped it about a dozen times on its way out.
const FUEL: u32 = 4096;

/// Operators and keywords from other languages, with what to say about them.
fn foreign_spelling(name: &str) -> Option<(&'static str, &'static str)> {
    match name {
        "and" => Some(("unknown operator `and`", "mimas spells this `&&`")),
        "or" => Some(("unknown operator `or`", "mimas spells this `||`")),
        "elif" => Some(("unknown keyword `elif`", "mimas spells this `else if`")),
        "as" => Some((
            "mimas has no `as` casts",
            "convert with a method instead, e.g. `.to_float()`",
        )),
        _ => None,
    }
}

/// What `node()` parsed: either a fully-formed stmt, or a bare expression whose role
/// (block-trailing yield vs expression statement) is decided by the caller. Assignments
/// commit to `Stmt` inside `node()` so they never surface as `MaybeYield`.
enum BlockElement {
    Stmt(Stmt),
    MaybeYield(Expr),
}

/// A binary operator, paired with how tightly it binds (loosest is 1).
enum BinaryOp {
    Logical(LogicalOp),
    Equality(EqualityOp),
    /// `in` when true, `!in` when false.
    In(bool),
    Eval(EvaluationOp),
}

impl BinaryOp {
    /// Returns the operator `kind` is, if it is one.
    fn of(kind: TokKind) -> Option<(Self, u8)> {
        if let Ok(op) = LogicalOp::try_from(kind) {
            return Some((Self::Logical(op), 1));
        }
        if let Ok(op) = EqualityOp::try_from(kind) {
            return Some((Self::Equality(op), 2));
        }
        if matches!(kind, TokKind::In | TokKind::NotIn) {
            return Some((Self::In(kind == TokKind::In), 3));
        }
        let op = EvaluationOp::try_from(kind).ok()?;
        let power = if op.is_binary() {
            4
        } else if op.is_bit_shift() {
            5
        } else if op.is_additive() {
            6
        } else {
            // the multiplicative ops, which bind tightest
            7
        };
        Some((Self::Eval(op), power))
    }
}
