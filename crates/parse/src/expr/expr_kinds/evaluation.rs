use crate::{Expr, ExprKind, IntoExpr, op};

/// A mathematical evaluation.
#[derive(Debug, PartialEq, Clone)]
pub struct Evaluation {
    /// The left hand side of the evaluation.
    pub left: Expr,
    /// The operator used in this evaluation.
    pub op: EvaluationOp,
    /// The right hand side of the evaluation.
    pub right: Expr,
}
impl Evaluation {
    /// Creates a new evaluation.
    pub(crate) fn new(left: Expr, op: EvaluationOp, right: Expr) -> Self {
        Self { left, op, right }
    }
}
impl From<Evaluation> for ExprKind {
    fn from(evaluation: Evaluation) -> Self {
        Self::Evaluation(evaluation)
    }
}
impl IntoExpr for Evaluation {}

op!(
    /// The various evaluation operations supported in mimas.
    EvaluationOp {
        Plus => Plus,
        Minus => Minus,
        Slash => Divide,
        Star => Multiply,
        TildeSlash => Div,
        Percent => Modulo,
        Ampersand => And,
        Pipe => Or,
        Caret => Xor,
        DoubleLeftCaret => BitShiftLeft,
        DoubleRightCaret => BitShiftRight,
    }
);

impl EvaluationOp {
    pub fn is_bitwise(&self) -> bool {
        self.is_binary() || self.is_bit_shift()
    }

    pub fn is_binary(&self) -> bool {
        matches!(
            self,
            EvaluationOp::And | EvaluationOp::Or | EvaluationOp::Xor
        )
    }

    pub(crate) fn is_bit_shift(&self) -> bool {
        matches!(
            self,
            EvaluationOp::BitShiftLeft | EvaluationOp::BitShiftRight
        )
    }

    pub(crate) fn is_additive(&self) -> bool {
        matches!(self, EvaluationOp::Plus | EvaluationOp::Minus)
    }

    pub(crate) fn is_multiplicative(&self) -> bool {
        matches!(
            self,
            EvaluationOp::Divide
                | EvaluationOp::Multiply
                | EvaluationOp::Div
                | EvaluationOp::Modulo
        )
    }
}

#[mutants::skip]
impl std::fmt::Display for Evaluation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{} {} {}", self.left, self.op, self.right))
    }
}
