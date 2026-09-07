use crate::{Expr, ExprKind, IntoExpr};
use itertools::Itertools;

#[derive(Debug, PartialEq, Clone)]
pub struct FString {
    pub parts: Vec<FStringPart>,
}

impl FString {
    pub(crate) fn new(parts: Vec<FStringPart>) -> Self {
        Self { parts }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum FStringPart {
    Literal(String),
    Expr(Expr),
}

impl From<FString> for ExprKind {
    fn from(f_string: FString) -> Self {
        Self::FString(f_string)
    }
}

impl IntoExpr for FString {}

#[mutants::skip]
impl std::fmt::Display for FString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!(
            "f\"{}\"",
            self.parts
                .iter()
                .map(|part| match part {
                    FStringPart::Literal(s) => s.clone(),
                    FStringPart::Expr(expr) => format!("{{{expr}}}"),
                })
                .join("")
        ))
    }
}
