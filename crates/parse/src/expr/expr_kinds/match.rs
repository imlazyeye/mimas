use crate::{Expr, ExprKind, IntoExpr, components::Pat};

/// A `match` expression.
#[derive(Debug, PartialEq, Clone)]
pub struct Match {
    pub identity: Expr,
    pub cases: Vec<MatchCase>,
    /// `!` terminator -- promises exhaustion. If reached at runtime, the program panics.
    pub panic_terminator: bool,
}

impl Match {
    pub(crate) fn new(identity: Expr, cases: Vec<MatchCase>, panic_terminator: bool) -> Self {
        Self {
            identity,
            cases,
            panic_terminator,
        }
    }
}

impl From<Match> for ExprKind {
    fn from(m: Match) -> Self {
        Self::Match(m)
    }
}
impl IntoExpr for Match {}

#[derive(Debug, PartialEq, Clone)]
pub struct MatchCase {
    pat: Pat,
    /// `if expr` guard -- only matched if the expression evaluates to true.
    guard: Option<Expr>,
    body: Expr,
}

impl MatchCase {
    pub(crate) fn new(pat: Pat, guard: Option<Expr>, body: Expr) -> Self {
        Self { pat, guard, body }
    }

    pub fn pat(&self) -> &Pat {
        &self.pat
    }

    pub fn guard(&self) -> Option<&Expr> {
        self.guard.as_ref()
    }

    pub fn body(&self) -> &Expr {
        &self.body
    }
}

#[mutants::skip]
impl std::fmt::Display for Match {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad("match<todo>")
    }
}
