use crate::{Expr, ExprKind, FieldKey, IntoExpr, expr::Ident, lex::TokKind};
use itertools::Itertools;

/// Representation of a literal in mimas, aka a constant compile-time value.
#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    /// true
    True,
    /// false
    False,
    /// null
    Null,
    /// unit as value
    Unit,
    /// A string literal
    String(String),
    /// Int
    Int(i64),
    Float(f64),
    /// A hex-format number
    Hex(String),
    /// An array literal ([0, 1, 2])
    Array(Vec<Expr>),
    /// A dictionary literal (~{a = 0, b = 0})
    Dictionary(Vec<DictField>),
    /// A struct literal (Foo { a: 0 })
    Struct(StructLiteral),
    /// A tuple literal ((0, 1))
    Tuple(Vec<Expr>),
}

pub type DictField = (Ident, Expr);

#[derive(Debug, PartialEq, Clone)]
pub struct StructLiteral {
    pub name: Expr,
    pub fields: Vec<(FieldKey, Expr)>,
}

impl From<Literal> for ExprKind {
    fn from(literal: Literal) -> Self {
        Self::Literal(literal)
    }
}
impl IntoExpr for Literal {}

#[mutants::skip]
impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::True => f.pad("true"),
            Literal::False => f.pad("false"),
            Literal::Null => f.pad("null"),
            Literal::Unit => f.pad("()"),
            Literal::String(s) => f.pad(&format!("\"{}\"", s)),
            Literal::Int(r) => f.pad(&r.to_string()),
            Literal::Float(r) => f.pad(&format!("{r}")),
            Literal::Hex(h) => f.pad(&h.to_string()),
            Literal::Array(members) => f.pad(&format!(
                "[{}]",
                members.iter().map(|member| member.to_string()).join(", ")
            )),
            Literal::Dictionary(fields) => f.pad(&format!(
                "~{{ {} }}",
                fields
                    .iter()
                    .map(|(Ident { lexeme, .. }, symbol)| format!("{lexeme} = {symbol}"))
                    .join(", ")
            )),
            Literal::Struct(StructLiteral { name, fields }) => f.pad(&format!(
                "{name} {{ {} }}",
                fields
                    .iter()
                    .map(|(name, value)| format!("{name} = {value}"))
                    .join(", ")
            )),
            Literal::Tuple(exprs) => f.pad(&format!(
                "({})",
                exprs.iter().map(|v| v.to_string()).join(", ")
            )),
        }
    }
}

impl TryFrom<TokKind<'_>> for Literal {
    type Error = ();

    fn try_from(value: TokKind<'_>) -> Result<Self, Self::Error> {
        match value {
            TokKind::True => Ok(Literal::True),
            TokKind::False => Ok(Literal::False),
            TokKind::Null => Ok(Literal::Null),
            TokKind::String(s) => Ok(Literal::String(chompy::utils::unescape(s, &['\\'], &['"']))),
            TokKind::Int(n) => Ok(Literal::Int(n)),
            TokKind::Float(n) => Ok(Literal::Float(n)),
            TokKind::Hex(n) => Ok(Literal::Hex(n.to_string())),
            _ => Err(()),
        }
    }
}
