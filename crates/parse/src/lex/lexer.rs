use std::sync::Arc;

use chompy::{
    define_error,
    diagnostics::{Builder, Result as ChompyResult},
    lex::{CharStream, Lex, Tok, UnterminatedString},
    utils::*,
};
use miette::NamedSource;

use crate::{
    errors::LexError,
    lex::{TokKind, TyKw},
};

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

#[derive(Debug, Clone)]
pub struct Lexer<'s> {
    source: &'s str,
    char_stream: CharStream<'s>,
    file_id: FileId,
    file_name: String,
}

impl<'s> Lexer<'s> {
    /// Creates a new Lexer, taking a string of mimas source.
    pub fn new(source: &'s str, file_id: FileId, file_name: String) -> Self {
        Self {
            source,
            char_stream: CharStream::new(source),
            file_id,
            file_name,
        }
    }

    pub(crate) fn file_name(&self) -> &str {
        &self.file_name
    }

    fn is_fstring_start(&self) -> bool {
        let bytes = self.source.as_bytes();
        let pos = self.char_stream.position();
        bytes.get(pos).copied() == Some(b'f') && bytes.get(pos + 1).copied() == Some(b'"')
    }

    /// Chomp a `/* ... */` block comment if one starts at the cursor. Nests like Rust's, so
    /// commenting out a region that already contains a block comment closes at the right `*/`.
    /// Returns `None` when the cursor isn't on a `/*` at all.
    fn construct_block_comment(&mut self) -> Option<ChompyResult<()>> {
        if !(self.char_stream.match_peek('/') && self.char_stream.match_peek('*')) {
            self.char_stream.reset_peeks();
            return None;
        }
        let start = self.char_stream.position();
        self.char_stream.chomp_peeks();

        // `prev` is cleared after each delimiter so a shared slash can't count twice --
        // `/*/` must not open-and-close, matching rustc.
        let mut depth = 1usize;
        let mut prev = None;
        loop {
            let Some(c) = self.char_stream.chomp() else {
                let location = Location::new(
                    self.file_id,
                    Span::new(start, self.char_stream.position().min(self.source.len())),
                );
                return Some(Err(UnterminatedBlockComment(location).into()));
            };
            match (prev, c) {
                (Some('/'), '*') => {
                    depth += 1;
                    prev = None;
                }
                (Some('*'), '/') => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(Ok(()));
                    }
                    prev = None;
                }
                _ => prev = Some(c),
            }
        }
    }

    /// Lex the body of an f-string `f"..."`. Caller has already chomped the `f`;
    /// the cursor must be on the opening `"`.
    ///
    /// Unlike chompy's `construct_string`, this tracks `{...}` interpolation
    /// depth so that a `"` *inside* an interp (e.g. `f"{m["k"]}"`) opens a
    /// nested string instead of terminating the f-string. The literal portion
    /// still honors `\"` and `{{` / `}}` escapes.
    fn construct_fstring(&mut self) -> ChompyResult<&'s str> {
        let file_id = self.file_id;
        let stream = &mut self.char_stream;
        let start = stream.position();
        let opening = stream.peek_move().expect("called with `\"` at the cursor");
        debug_assert_eq!(opening, '"');

        let mut brace_depth: u32 = 0;
        let mut in_escape = false;
        loop {
            let Some(c) = stream.peek_move() else {
                let location = Location::new(file_id, Span::new(start, stream.peek_position()));
                return Err(UnterminatedString(location).into());
            };
            if brace_depth == 0 {
                if in_escape {
                    in_escape = false;
                    continue;
                }
                match c {
                    '\\' => in_escape = true,
                    '"' => {
                        let open_w = opening.len_utf8();
                        let close_w = c.len_utf8();
                        let slice =
                            stream.slice((start + open_w)..(stream.peek_position() - close_w));
                        stream.chomp_peeks();
                        return Ok(slice);
                    }
                    '{' => {
                        if stream.peek() == Some('{') {
                            stream.advance();
                        } else {
                            brace_depth = 1;
                        }
                    }
                    '}' if stream.peek() == Some('}') => stream.advance(),
                    _ => {}
                }
            } else {
                match c {
                    '{' => brace_depth += 1,
                    '}' => brace_depth -= 1,
                    '"' => {
                        let mut esc = false;
                        loop {
                            let Some(ic) = stream.peek_move() else {
                                let location = Location::new(
                                    file_id,
                                    Span::new(start, stream.peek_position()),
                                );
                                return Err(UnterminatedString(location).into());
                            };
                            if esc {
                                esc = false;
                                continue;
                            }
                            match ic {
                                '\\' => esc = true,
                                '"' => break,
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

impl<'s> Iterator for Lexer<'s> {
    type Item = shared::Result<Tok<TokKind<'s>>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.lex() {
            Ok(Some(item)) => Some(Ok(item)),
            Err(diag) => {
                let src = NamedSource::new(self.file_name.clone(), Arc::<str>::from(self.source));
                // todo, use diag.location() once the chompy pin carries Diag locations;
                // until then the cursor at failure time is the best anchor we have
                let pos = self.char_stream.position().min(self.source.len());
                let at = if pos >= self.source.len() {
                    self.source.len().saturating_sub(1)..self.source.len()
                } else {
                    pos..pos + 1
                };
                Some(Err(LexError {
                    src,
                    diag,
                    at: Some(at.into()),
                }
                .into()))
            }
            _ => None,
        }
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
        loop {
            if self.char_stream.match_chomp_with(|c| c.is_whitespace())
                || self.construct_comment(&["//"]).is_some()
            {
                continue;
            }
            match self.construct_block_comment() {
                Some(Ok(())) => continue,
                Some(Err(diag)) => return Err(diag),
                None => break,
            }
        }
        let start_pos = self.char_stream.position();
        let kind = if let Some(hex) = self.construct_hex("0x") {
            TokKind::Hex(hex?)
        } else if let Some(float) = self.construct_float(true, true) {
            TokKind::Float(float)
        } else if let Some(int) = self.construct_integer(true) {
            TokKind::Int(int)
        } else if self.is_fstring_start() {
            // chomp the leading `f`
            self.char_stream.chomp();
            let string = self.construct_fstring()?;
            TokKind::FString(string)
        } else if let Some(string) = self.construct_string(&['"'], &['\\']) {
            TokKind::String(string?)
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
                    if self.match_chomp('=') {
                        TokKind::BangEqual
                    } else if self.chomp_pattern("in") {
                        TokKind::NotIn
                    } else {
                        TokKind::Bang
                    }
                }
                '/' => {
                    if self.match_chomp('=') {
                        TokKind::SlashEqual
                    } else {
                        TokKind::Slash
                    }
                }
                invalid => {
                    // this is chill, I promise
                    let tmp = Box::leak(Box::new([0u8; 4]));
                    let invalid = invalid.encode_utf8(tmp);
                    TokKind::Invalid(invalid)
                }
            }
        };

        Ok(Some(Tok::new(
            kind,
            Location::new(
                self.file_id,
                Span::new(start_pos, self.char_stream.position()),
            ),
        )))
    }
}
