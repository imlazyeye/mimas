use crate::{Expr, IntoStmt, StmtKind, op};

/// Representation of an assignment statement in mimas.
#[derive(Debug, PartialEq, Clone)]
pub struct Assignment {
    /// The left hand side of the assignment, aka the target.
    pub left: Expr,
    /// The operator used in this assignment.
    pub op: AssignmentOp,
    /// The right hand side of the assignment, aka the value.
    pub right: Expr,
}
impl Assignment {
    /// Creates a new assignment.
    pub(crate) fn new(left: Expr, op: AssignmentOp, right: Expr) -> Self {
        Self { left, op, right }
    }
}
impl From<Assignment> for StmtKind {
    fn from(assignment: Assignment) -> Self {
        Self::Assignment(assignment)
    }
}
impl IntoStmt for Assignment {}

op!(
    /// The various assignment operations supported in mimas.
    AssignmentOp {
        Equal => Identity,
        PlusEqual => PlusEqual,
        MinusEqual => MinusEqual,
        StarEqual => StarEqual,
        SlashEqual => SlashEqual,
        CaretEqual => XorEqual,
        PipeEqual => OrEqual,
        AmpersandEqual => AndEqual,
        DoubleHookEqual => NullCoalescenceEqual,
        PercentEqual => ModEqual,
        TildeSlashEqual => DivEqual,
    }
);

#[mutants::skip]
impl std::fmt::Display for Assignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{} {} {}", self.left, self.op, self.right))
    }
}
