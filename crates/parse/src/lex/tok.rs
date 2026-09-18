use crate::{Literal, UnaryOp};
use chompy::lex::TokenKind;
use std::fmt::Display;

/// An individual tok of mimas.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokKind<'s> {
    Match,
    Break,
    Collect,
    Return,
    Raise,
    Absolve,
    Struct,
    Impl,
    Pact,
    Const,
    Pub,
    Module,
    At,
    Colon,
    DoubleColon,
    Dot,
    DoubleDot,
    DoubleDotEqual,
    LeftSquare,
    RightSquare,
    Enum,
    LeftBrace,
    RightBrace,
    TildeLeftBrace,
    LeftParenthesis,
    RightParenthesis,
    Comma,
    Ampersand,
    DoubleAmpersand,
    DoublePipe,
    Caret,
    Equal,
    DoubleEqual,
    BangEqual,
    Fn,
    SelfKeyword,
    Percent,
    PercentEqual,
    TildeSlash,
    TildeSlashEqual,
    Slash,
    Star,
    True,
    False,
    Plus,
    Minus,
    Bang,
    Hook,
    DoubleHook,
    DoubleHookEqual,
    HookDot,
    HookLeftSquare,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Pipe,
    SemiColon,
    If,
    Else,
    For,
    NotIn,
    In,
    Loop,
    While,
    Let,
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    PipeEqual,
    AmpersandEqual,
    CaretEqual,
    Tilde,
    DoubleLeftCaret,
    DoubleRightCaret,
    Continue,
    Null,
    Use,
    Arrow,
    FatArrow,
    TyKw(TyKw),
    Ident(&'s str),
    Int(i64),
    Float(f64),
    String(&'s str),
    FString(&'s str),
    Hex(&'s str),
    Invalid(&'s str),
    /// Ends every token stream the parser reads. The lexer never produces one.
    Eof,
}

#[mutants::skip]
impl Display for TokKind<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TokKind::Match => "match",
            TokKind::Break => "break",
            TokKind::Collect => "collect",
            TokKind::Return => "return",
            TokKind::Raise => "raise",
            TokKind::Absolve => "absolve",
            TokKind::Struct => "struct",
            TokKind::Pact => "pact",
            TokKind::Impl => "impl",
            TokKind::Const => "const",
            TokKind::Pub => "pub",
            TokKind::Module => "module",
            TokKind::At => "@",
            TokKind::Colon => ":",
            TokKind::DoubleColon => "::",
            TokKind::Dot => ".",
            TokKind::DoubleDot => "..",
            TokKind::DoubleDotEqual => "..=",
            TokKind::LeftSquare => "[",
            TokKind::RightSquare => "]",
            TokKind::Enum => "enum",
            TokKind::LeftBrace => "{",
            TokKind::RightBrace => "}",
            TokKind::TildeLeftBrace => "~{",
            TokKind::LeftParenthesis => "(",
            TokKind::RightParenthesis => ")",
            TokKind::Comma => ",",
            TokKind::Ampersand => "&",
            TokKind::DoubleAmpersand => "&&",
            TokKind::DoublePipe => "||",
            TokKind::Caret => "^",
            TokKind::Equal => "=",
            TokKind::DoubleEqual => "==",
            TokKind::BangEqual => "!=",
            TokKind::Fn => "fn",
            TokKind::SelfKeyword => "self",
            TokKind::Percent => "%",
            TokKind::PercentEqual => "%=",
            TokKind::TildeSlash => "~/",
            TokKind::TildeSlashEqual => "~/=",
            TokKind::Slash => "/",
            TokKind::Star => "*",
            TokKind::True => "true",
            TokKind::False => "false",
            TokKind::Plus => "+",
            TokKind::Minus => "-",
            TokKind::Bang => "!",
            TokKind::Hook => "?",
            TokKind::DoubleHook => "??",
            TokKind::DoubleHookEqual => "??=",
            TokKind::HookDot => "?.",
            TokKind::HookLeftSquare => "?[",
            TokKind::Greater => ">",
            TokKind::GreaterEqual => ">=",
            TokKind::Less => "<",
            TokKind::LessEqual => "<=",
            TokKind::Pipe => "|",
            TokKind::SemiColon => ";",
            TokKind::If => "if",
            TokKind::Else => "else",
            TokKind::For => "for",
            TokKind::In => "in",
            TokKind::NotIn => "!in",
            TokKind::Loop => "loop",
            TokKind::While => "while",
            TokKind::Let => "let",
            TokKind::PlusEqual => "+=",
            TokKind::MinusEqual => "-=",
            TokKind::StarEqual => "*=",
            TokKind::SlashEqual => "/=",
            TokKind::PipeEqual => "|=",
            TokKind::AmpersandEqual => "&=",
            TokKind::CaretEqual => "^=",
            TokKind::Tilde => "~",
            TokKind::DoubleLeftCaret => "<<",
            TokKind::DoubleRightCaret => ">>",
            TokKind::Continue => "continue",
            TokKind::Null => "null",
            TokKind::Use => "use",
            TokKind::Arrow => "->",
            TokKind::FatArrow => "=>",
            TokKind::TyKw(tym) => return f.pad(&tym.to_string()),
            TokKind::Ident(iden) => iden,
            TokKind::Int(r) => return f.pad(&r.to_string()),
            TokKind::Float(r) => return f.pad(&r.to_string()),
            TokKind::String(s) => return f.pad(&format!("\"{s}\"")),
            TokKind::FString(s) => return f.pad(&format!("f\"{s}\"")),
            TokKind::Hex(hex) => hex,
            TokKind::Invalid(_) => "INVALID_TOKEN",
            TokKind::Eof => return f.pad("end of input"),
        };
        f.pad(s)
    }
}

impl TokenKind for TokKind<'_> {}

#[derive(Debug, Copy, PartialEq, Clone)]
pub enum TyKw {
    Int,
    Float,
    Str,
    Bool,
}

#[mutants::skip]
impl std::fmt::Display for TyKw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TyKw::Int => f.pad("int"),
            TyKw::Float => f.pad("float"),
            TyKw::Str => f.pad("str"),
            TyKw::Bool => f.pad("bool"),
        }
    }
}

// Parsing logic
impl TokKind<'_> {
    /// Returns if this can start a statement.
    pub(crate) fn starts_stmt(self) -> bool {
        self.is_stmt_keyword() || self.starts_expr()
    }

    /// Returns if this can start an item.
    pub(crate) fn starts_item(self) -> bool {
        matches!(
            self,
            TokKind::Pub
                | TokKind::Fn
                | TokKind::Struct
                | TokKind::Pact
                | TokKind::Enum
                | TokKind::Impl
                | TokKind::Const
                | TokKind::Use
        )
    }

    /// Returns if this can start an expression.
    pub(crate) fn starts_expr(self) -> bool {
        Literal::try_from(self).is_ok()
            || UnaryOp::try_from(self).is_ok()
            || matches!(
                self,
                TokKind::Ident(_)
                    | TokKind::SelfKeyword
                    | TokKind::TyKw(_)
                    | TokKind::FString(_)
                    | TokKind::LeftParenthesis
                    | TokKind::LeftSquare
                    | TokKind::LeftBrace
                    | TokKind::TildeLeftBrace
                    | TokKind::Pipe
                    | TokKind::DoublePipe
                    | TokKind::Loop
                    | TokKind::While
                    | TokKind::For
                    | TokKind::If
                    | TokKind::Match
                    | TokKind::Return
                    | TokKind::Raise
                    | TokKind::Break
                    | TokKind::Collect
                    | TokKind::Continue
            )
    }

    /// Returns if this can start a pattern.
    pub(crate) fn starts_pattern(self) -> bool {
        Literal::try_from(self).is_ok()
            || matches!(
                self,
                TokKind::Ident(_) | TokKind::LeftParenthesis | TokKind::Minus
            )
    }

    /// Returns if this can start a type annotation.
    pub(crate) fn starts_annotation(self) -> bool {
        matches!(
            self,
            TokKind::Ident(_)
                | TokKind::TyKw(_)
                | TokKind::LeftParenthesis
                | TokKind::LeftSquare
                | TokKind::TildeLeftBrace
        )
    }

    /// Returns if this is an identifier.
    pub(crate) fn is_ident(self) -> bool {
        matches!(self, TokKind::Ident(_))
    }

    /// Returns if this is a keyword that starts a statement wherever it shows up. Recovery
    /// never skips these.
    pub(crate) fn is_stmt_keyword(self) -> bool {
        matches!(self, TokKind::Let | TokKind::Module) || self.starts_item()
    }

    /// Returns if this closes a group.
    pub(crate) fn is_closer(self) -> bool {
        matches!(
            self,
            TokKind::RightParenthesis | TokKind::RightSquare | TokKind::RightBrace
        )
    }

    /// Returns if this is a token recovery leaves where it is, for whatever's waiting on it.
    pub(crate) fn is_anchor(self) -> bool {
        self.ends_list() || self == TokKind::Comma
    }

    /// Returns if a list that finds this where an element should start is over. These can't
    /// be part of a list unless an element takes them, so something further out is waiting
    /// for each.
    pub(crate) fn ends_list(self) -> bool {
        self.is_boundary()
            || matches!(
                self,
                TokKind::Equal
                    | TokKind::Arrow
                    | TokKind::FatArrow
                    | TokKind::LeftBrace
                    | TokKind::In
                    | TokKind::Else
            )
    }

    /// Returns if no skip inside a list runs past this.
    pub(crate) fn is_boundary(self) -> bool {
        self.ends_stmt() || self.is_closer()
    }

    /// Returns if a broken statement never runs past this.
    pub(crate) fn ends_stmt(self) -> bool {
        matches!(
            self,
            TokKind::SemiColon | TokKind::RightBrace | TokKind::Eof
        ) || self.is_stmt_keyword()
    }
}
