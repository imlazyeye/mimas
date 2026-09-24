use chompy::{
    define_error,
    diagnostics::{Builder, DiagBox, Result as ChompyResult},
    lex::{CharStream, Lex, Tok, UnterminatedString},
    utils::*,
};

use crate::lex::{TokKind, TyKw};

/// Lexing error for a `/*` that never meets its `*/`.
pub struct UnterminatedBlockComment(pub Location);
define_error!(
    UnterminatedBlockComment {
        fn build(&self, builder: Builder) -> Builder {
            builder.label(self.0.primary("this block comment was never closed with `*/`"))
        }

        fn location(&self) -> Location {
            self.0
        }
    }
);

/// Lexing error for a whole number too big to hold.
pub struct IntegerOutOfRange(pub Location);
define_error!(
    IntegerOutOfRange {
        fn build(&self, builder: Builder) -> Builder {
            builder.label(self.0.primary("this number doesn't fit in an int"))
        }

        fn location(&self) -> Location {
            self.0
        }
    }
);

/// Whether `source` opens with a `module` declaration, past any comments.
pub fn is_module(source: &str) -> bool {
    let mut kinds = Lexer::new(source, 0, String::new())
        .map(|tok| tok.kind)
        .filter(|kind| !kind.is_comment());
    matches!(kinds.next(), Some(TokKind::Module))
}

#[derive(Debug)]
pub struct Lexer<'s> {
    source: &'s str,
    char_stream: CharStream<'s>,
    file_id: FileId,
    file_name: String,
    offset: usize,
    errors: Vec<(DiagBox, Location)>,
    cut_short: bool,
}

impl<'s> Lexer<'s> {
    /// Creates a new Lexer, taking a string of mimas source.
    pub fn new(source: &'s str, file_id: FileId, file_name: String) -> Self {
        Self {
            source,
            char_stream: CharStream::new(source),
            file_id,
            file_name,
            offset: 0,
            errors: vec![],
            cut_short: false,
        }
    }

    /// Shifts every location the lexer produces by `offset`. For lexing a slice of a larger
    /// source (an f-string's interpolation) with spans that still point into the whole thing.
    pub(crate) fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    pub(crate) fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Takes the errors found since the last call, each with where it happened.
    pub fn take_errors(&mut self) -> Vec<(DiagBox, Location)> {
        std::mem::take(&mut self.errors)
    }

    /// Whether a string or comment that never closed took the rest of the source with it.
    pub(crate) fn cut_short(&self) -> bool {
        self.cut_short
    }

    /// Where the source ends.
    pub(crate) fn end(&self) -> Location {
        self.location(self.source.len(), self.source.len())
    }

    /// `start..end` of our own source, as a location in the whole thing.
    fn location(&self, start: usize, end: usize) -> Location {
        Location::new(
            self.file_id,
            Span::new(self.offset + start, self.offset + end),
        )
    }

    /// Reports whatever `start..body` opened as never closed. Only the opener is marked (the
    /// rest of the file would be otherwise).
    fn unterminated(&mut self, start: usize, body: usize, diag: impl FnOnce(Location) -> DiagBox) {
        let location = self.location(start, body);
        self.errors.push((diag(location), location));
        self.cut_short = true;
    }

    /// Chomps the string or f-string opened at `start`. `body` is everything after the opening
    /// quote, and `end` is where in it the closing quote is (when there is one). Returns what's
    /// between the quotes.
    fn construct_quoted(&mut self, start: usize, body: &'s str, end: Option<usize>) -> &'s str {
        fn chomp_to(stream: &mut CharStream, end: usize) {
            while stream.position() < end {
                stream.chomp();
            }
        }
        let body_start = self.source.len() - body.len();
        match end {
            Some(end) => {
                chomp_to(&mut self.char_stream, body_start + end + 1);
                &body[..end]
            }
            None => {
                self.unterminated(start, body_start, |location| {
                    UnterminatedString(location).into()
                });
                chomp_to(&mut self.char_stream, self.source.len());
                body
            }
        }
    }

    /// Chomps the rest of the block comment whose `/*` is at `start`. Nests like Rust's, so
    /// commenting out a region that already contains a block comment closes at the right `*/`.
    /// Returns the whole comment.
    fn construct_block_comment(&mut self, start: usize) -> &'s str {
        // `prev` is cleared after each delimiter so a shared slash can't count twice --
        // `/*/` must not open-and-close, matching rustc.
        let mut depth = 1usize;
        let mut prev = None;
        while depth > 0 {
            let Some(c) = self.char_stream.chomp() else {
                self.unterminated(start, start + 2, |location| {
                    UnterminatedBlockComment(location).into()
                });
                break;
            };
            match (prev, c) {
                (Some('/'), '*') => {
                    depth += 1;
                    prev = None;
                }
                (Some('*'), '/') => {
                    depth -= 1;
                    prev = None;
                }
                _ => prev = Some(c),
            }
        }
        &self.source[start..self.char_stream.position()]
    }
}

impl<'s> Iterator for Lexer<'s> {
    type Item = Tok<TokKind<'s>>;

    fn next(&mut self) -> Option<Self::Item> {
        // chompy's trait makes `lex` a result, but ours has no way to fail
        self.lex().ok().flatten()
    }
}

impl<'s> Lex<'s, Tok<TokKind<'s>>, TokKind<'s>> for Lexer<'s> {
    fn source(&self) -> &'s str {
        self.source
    }

    fn file_id(&self) -> FileId {
        self.file_id
    }

    fn char_stream(&mut self) -> &mut CharStream<'s> {
        &mut self.char_stream
    }

    fn lex(&mut self) -> ChompyResult<Option<Tok<TokKind<'s>>>> {
        // whitespace in one go -- massive amounts of whitespace can cause enormously deep stacks
        while self.char_stream.match_chomp_with(|c| c.is_whitespace()) {}
        let start = self.char_stream.position();
        let rest = &self.source[start..];
        let kind = if let Some(hex) = self.construct_hex("0x") {
            match hex {
                Ok(hex) => TokKind::Hex(hex),
                // a `0x` with no digits after it is still a hex literal, just an empty one
                Err(diag) => {
                    let location = self.location(start, self.char_stream.position());
                    self.errors.push((diag, location));
                    TokKind::Hex("")
                }
            }
        } else if let Some(float) = self.construct_float(true, true) {
            TokKind::Float(float)
        } else if rest.starts_with(|c: char| c.is_ascii_digit()) {
            match self.construct_integer(true) {
                Some(int) => TokKind::Int(int),
                // on a digit, the only way for that to fail is a number too big to hold
                None => {
                    let location = self.location(start, self.char_stream.position());
                    self.errors
                        .push((IntegerOutOfRange(location).into(), location));
                    TokKind::Int(0)
                }
            }
        } else if let Some(body) = rest.strip_prefix("f\"") {
            TokKind::FString(self.construct_quoted(start, body, fstring_end(body)))
        } else if let Some(body) = rest.strip_prefix('"') {
            TokKind::String(self.construct_quoted(start, body, string_end(body)))
        } else if let Some(ident) = self.construct_ident() {
            match ident {
                "self" => TokKind::SelfKeyword,
                "let" => TokKind::Let,
                "return" => TokKind::Return,
                "raise" => TokKind::Raise,
                "absolve" => TokKind::Absolve,
                "if" => TokKind::If,
                "else" => TokKind::Else,
                "null" => TokKind::Null,
                "struct" => TokKind::Struct,
                "pact" => TokKind::Pact,
                "impl" => TokKind::Impl,
                "const" => TokKind::Const,
                "pub" => TokKind::Pub,
                "module" => TokKind::Module,
                "break" => TokKind::Break,
                "collect" => TokKind::Collect,
                "fn" => TokKind::Fn,
                "true" => TokKind::True,
                "false" => TokKind::False,
                "for" => TokKind::For,
                "in" => TokKind::In,
                "match" => TokKind::Match,
                "continue" => TokKind::Continue,
                "enum" => TokKind::Enum,
                "loop" => TokKind::Loop,
                "while" => TokKind::While,
                "use" => TokKind::Use,
                "int" => TokKind::TyKw(TyKw::Int),
                "float" => TokKind::TyKw(TyKw::Float),
                "str" => TokKind::TyKw(TyKw::Str),
                "bool" => TokKind::TyKw(TyKw::Bool),
                "" => TokKind::TyKw(TyKw::Bool),

                lexeme => TokKind::Ident(lexeme),
            }
        } else {
            let Some(chr) = self.chomp() else {
                return Ok(None);
            };
            match chr {
                '.' => {
                    if self.match_chomp('.') {
                        if self.match_chomp('=') {
                            TokKind::DoubleDotEqual
                        } else {
                            TokKind::DoubleDot
                        }
                    } else {
                        TokKind::Dot
                    }
                }
                ',' => TokKind::Comma,
                ';' => TokKind::SemiColon,
                '(' => TokKind::LeftParenthesis,
                ')' => TokKind::RightParenthesis,
                '{' => TokKind::LeftBrace,
                '}' => TokKind::RightBrace,
                '[' => TokKind::LeftSquare,
                ']' => TokKind::RightSquare,
                '@' => TokKind::At,
                '=' => {
                    if self.match_chomp('=') {
                        TokKind::DoubleEqual
                    } else if self.match_chomp('>') {
                        TokKind::FatArrow
                    } else {
                        TokKind::Equal
                    }
                }
                '-' => {
                    if self.match_chomp('=') {
                        TokKind::MinusEqual
                    } else if self.match_chomp('>') {
                        TokKind::Arrow
                    } else {
                        TokKind::Minus
                    }
                }
                '+' => {
                    if self.match_chomp('=') {
                        TokKind::PlusEqual
                    } else {
                        TokKind::Plus
                    }
                }
                '*' => {
                    if self.match_chomp('=') {
                        TokKind::StarEqual
                    } else {
                        TokKind::Star
                    }
                }
                '<' => {
                    if self.match_chomp('=') {
                        TokKind::LessEqual
                    } else if self.match_chomp('<') {
                        TokKind::DoubleLeftCaret
                    } else {
                        TokKind::Less
                    }
                }
                '>' => {
                    if self.match_chomp('=') {
                        TokKind::GreaterEqual
                    } else if self.match_chomp('>') {
                        TokKind::DoubleRightCaret
                    } else {
                        TokKind::Greater
                    }
                }
                '&' => {
                    if self.match_chomp('&') {
                        TokKind::DoubleAmpersand
                    } else if self.match_chomp('=') {
                        TokKind::AmpersandEqual
                    } else {
                        TokKind::Ampersand
                    }
                }
                '|' => {
                    if self.match_chomp('|') {
                        TokKind::DoublePipe
                    } else if self.match_chomp('=') {
                        TokKind::PipeEqual
                    } else {
                        TokKind::Pipe
                    }
                }
                ':' => {
                    if self.match_chomp(':') {
                        TokKind::DoubleColon
                    } else {
                        TokKind::Colon
                    }
                }
                '~' => {
                    if self.match_chomp('/') {
                        if self.match_chomp('=') {
                            TokKind::TildeSlashEqual
                        } else {
                            TokKind::TildeSlash
                        }
                    } else if self.match_chomp('{') {
                        TokKind::TildeLeftBrace
                    } else {
                        TokKind::Tilde
                    }
                }
                '%' => {
                    if self.match_chomp('=') {
                        TokKind::PercentEqual
                    } else {
                        TokKind::Percent
                    }
                }
                '?' => {
                    if self.match_chomp('?') {
                        if self.match_chomp('=') {
                            TokKind::DoubleHookEqual
                        } else {
                            TokKind::DoubleHook
                        }
                    } else if self.match_chomp('.') {
                        TokKind::HookDot
                    } else if self.match_chomp('[') {
                        TokKind::HookLeftSquare
                    } else {
                        TokKind::Hook
                    }
                }
                '^' => {
                    if self.match_chomp('=') {
                        TokKind::CaretEqual
                    } else {
                        TokKind::Caret
                    }
                }
                '!' => {
                    // `!in` is the operator only when the `in` ends there: `!inside` is `!`
                    // `inside`
                    let rest = &self.source()[self.char_stream().peek_position()..];
                    let not_in = rest.starts_with("in")
                        && !rest[2..].starts_with(|c: char| c.is_alphanumeric() || c == '_');
                    if self.match_chomp('=') {
                        TokKind::BangEqual
                    } else if not_in && self.chomp_pattern("in") {
                        TokKind::NotIn
                    } else {
                        TokKind::Bang
                    }
                }
                '/' => {
                    if self.match_chomp('/') {
                        let is_doc = self.match_chomp('/');
                        let stream = &mut self.char_stream;
                        loop {
                            while stream.match_chomp_with(|c| c != '\n' && c != '\r') {}
                            // carry on into the next line if it's the same kind of comment
                            stream.match_peek('\r');
                            stream.match_peek('\n');
                            stream.peek_while(|c| c == ' ' || c == '\t');
                            if stream.match_peek('/')
                                && stream.match_peek('/')
                                && stream.match_peek('/') == is_doc
                            {
                                stream.chomp_peeks();
                            } else {
                                stream.reset_peeks();
                                break;
                            }
                        }
                        let text = &self.source[start..self.char_stream.position()];
                        if is_doc {
                            TokKind::DocComment(text)
                        } else {
                            TokKind::Comment(text)
                        }
                    } else if self.match_chomp('*') {
                        TokKind::Comment(self.construct_block_comment(start))
                    } else if self.match_chomp('=') {
                        TokKind::SlashEqual
                    } else {
                        TokKind::Slash
                    }
                }
                _ => TokKind::Invalid(&self.source[start..self.char_stream.position()]),
            }
        };

        let location = self.location(start, self.char_stream.position());
        Ok(Some(Tok::new(kind, location)))
    }
}

/// Where the string whose body is the start of `text` has its closing quote.
fn string_end(text: &str) -> Option<usize> {
    let mut chars = text.char_indices();
    while let Some((at, c)) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '"' => return Some(at),
            _ => {}
        }
    }
    None
}

/// Where the f-string whose body is the start of `text` has its closing quote. A `"` inside an
/// interpolation (`f"{m["k"]}"`) opens a nested string rather than closing the f-string.
fn fstring_end(text: &str) -> Option<usize> {
    let mut at = 0;
    loop {
        let rest = &text[at..];
        let mut chars = rest.chars();
        match chars.next()? {
            '"' => return Some(at),
            '\\' => at += 1 + chars.next().map_or(0, char::len_utf8),
            '{' if rest.starts_with("{{") => at += 2,
            '{' => match interp_end(&rest[1..]) {
                Some(end) => at += 1 + end + 1,
                // an interpolation that never closes holds no strings, so the next quote is ours
                None => return rest.find('"').map(|quote| at + quote),
            },
            c => at += c.len_utf8(),
        }
    }
}

/// Where the `}` closing an f-string interpolation is, given the text right after its `{`.
/// Braces nest, and one inside a nested string doesn't count.
pub(crate) fn interp_end(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut at = 0;
    loop {
        let c = text[at..].chars().next()?;
        at += c.len_utf8();
        match c {
            '}' if depth == 0 => return Some(at - 1),
            '}' => depth -= 1,
            '{' => depth += 1,
            '"' => {
                at += string_end(&text[at..])? + 1;
            }
            _ => {}
        }
    }
}
