use parse::{AssignmentOp, EqualityOp, EvaluationOp, LogicalOp};

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mult,
    Div,
    IDiv,
    Mod,
    Identity,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
    And,
    Or,
    Xor,
    BitAnd,
    BitOr,
    BitXor,
    BitShiftLeft,
    BitShiftRight,
    Coalesce,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Scalar {
    Int(i64),
    Bool(bool),
    Float(f64),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BinFault {
    Overflow,
    DivByZero,
    ModByZero,
    InvalidShift,
    Type,
}

impl BinOp {
    pub fn eval_int(self, a: i64, b: i64) -> Result<Scalar, BinFault> {
        use BinOp::*;
        Ok(match self {
            Add => Scalar::Int(a.checked_add(b).ok_or(BinFault::Overflow)?),
            Sub => Scalar::Int(a.checked_sub(b).ok_or(BinFault::Overflow)?),
            Mult => Scalar::Int(a.checked_mul(b).ok_or(BinFault::Overflow)?),
            Div => Scalar::Float(a as f64 / b as f64),
            IDiv if b == 0 => return Err(BinFault::DivByZero),
            IDiv => Scalar::Int(a.checked_div(b).ok_or(BinFault::Overflow)?),
            Mod if b == 0 => return Err(BinFault::ModByZero),
            Mod => Scalar::Int(a.checked_rem(b).ok_or(BinFault::Overflow)?),
            Identity => Scalar::Bool(a == b),
            NotEqual => Scalar::Bool(a != b),
            LessThan => Scalar::Bool(a < b),
            LessEqual => Scalar::Bool(a <= b),
            GreaterThan => Scalar::Bool(a > b),
            GreaterEqual => Scalar::Bool(a >= b),
            BitAnd => Scalar::Int(a & b),
            BitOr => Scalar::Int(a | b),
            BitXor => Scalar::Int(a ^ b),
            BitShiftLeft => {
                let s: u32 = b.try_into().map_err(|_| BinFault::InvalidShift)?;
                Scalar::Int(a.checked_shl(s).ok_or(BinFault::InvalidShift)?)
            }
            BitShiftRight => {
                let s: u32 = b.try_into().map_err(|_| BinFault::InvalidShift)?;
                Scalar::Int(a.checked_shr(s).ok_or(BinFault::InvalidShift)?)
            }
            And | Or | Xor | Coalesce => return Err(BinFault::Type),
        })
    }

    pub fn eval_float(self, a: f64, b: f64) -> Result<Scalar, BinFault> {
        use BinOp::*;
        Ok(match self {
            Add => Scalar::Float(a + b),
            Sub => Scalar::Float(a - b),
            Mult => Scalar::Float(a * b),
            Div => Scalar::Float(a / b),
            IDiv => Scalar::Float((a / b).floor()),
            Mod => Scalar::Float(a % b),
            Identity => Scalar::Bool(a == b),
            NotEqual => Scalar::Bool(a != b),
            LessThan => Scalar::Bool(a < b),
            LessEqual => Scalar::Bool(a <= b),
            GreaterThan => Scalar::Bool(a > b),
            GreaterEqual => Scalar::Bool(a >= b),
            And | Or | Xor | BitAnd | BitOr | BitXor | BitShiftLeft | BitShiftRight | Coalesce => {
                return Err(BinFault::Type);
            }
        })
    }
}

#[mutants::skip]
impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOp::Add => f.pad("add"),
            BinOp::Sub => f.pad("sub"),
            BinOp::Mult => f.pad("mult"),
            BinOp::Div => f.pad("div"),
            BinOp::IDiv => f.pad("idiv"),
            BinOp::Mod => f.pad("mod"),
            BinOp::Identity => f.pad("equal"),
            BinOp::NotEqual => f.pad("not_equal"),
            BinOp::LessThan => f.pad("less_than"),
            BinOp::LessEqual => f.pad("less_equal"),
            BinOp::GreaterThan => f.pad("greater_than"),
            BinOp::GreaterEqual => f.pad("greater_equal"),
            BinOp::And => f.pad("and"),
            BinOp::Or => f.pad("or"),
            BinOp::Xor => f.pad("xor"),
            BinOp::BitAnd => f.pad("bit_and"),
            BinOp::BitOr => f.pad("bit_or"),
            BinOp::BitXor => f.pad("bit_xor"),
            BinOp::BitShiftLeft => f.pad("bit_shift_left"),
            BinOp::BitShiftRight => f.pad("bit_shift_right"),
            BinOp::Coalesce => f.pad("null_coalesce"),
        }
    }
}

impl From<EvaluationOp> for BinOp {
    fn from(value: EvaluationOp) -> Self {
        match value {
            EvaluationOp::Plus => BinOp::Add, // todo
            EvaluationOp::Minus => BinOp::Sub,
            EvaluationOp::Multiply => BinOp::Mult,
            EvaluationOp::Divide => BinOp::Div,
            EvaluationOp::Div => BinOp::IDiv,
            EvaluationOp::Modulo => BinOp::Mod,
            EvaluationOp::And => BinOp::BitAnd,
            EvaluationOp::Or => BinOp::BitOr,
            EvaluationOp::Xor => BinOp::BitXor,
            EvaluationOp::BitShiftLeft => BinOp::BitShiftLeft,
            EvaluationOp::BitShiftRight => BinOp::BitShiftRight,
        }
    }
}

impl From<EqualityOp> for BinOp {
    fn from(value: EqualityOp) -> Self {
        match value {
            EqualityOp::Equal => BinOp::Identity,
            EqualityOp::NotEqual => BinOp::NotEqual,
            EqualityOp::Greater => BinOp::GreaterThan,
            EqualityOp::GreaterOrEqual => BinOp::GreaterEqual,
            EqualityOp::Less => BinOp::LessThan,
            EqualityOp::LessOrEqual => BinOp::LessEqual,
        }
    }
}

impl From<LogicalOp> for BinOp {
    fn from(value: LogicalOp) -> Self {
        match value {
            LogicalOp::And => BinOp::And,
            LogicalOp::Or => BinOp::Or,
        }
    }
}

impl From<AssignmentOp> for BinOp {
    fn from(value: AssignmentOp) -> Self {
        match value {
            AssignmentOp::Identity => BinOp::Identity,
            AssignmentOp::PlusEqual => BinOp::Add,
            AssignmentOp::MinusEqual => BinOp::Sub,
            AssignmentOp::StarEqual => BinOp::Mult,
            AssignmentOp::SlashEqual => BinOp::Div,
            AssignmentOp::XorEqual => BinOp::BitXor,
            AssignmentOp::OrEqual => BinOp::BitOr,
            AssignmentOp::AndEqual => BinOp::BitAnd,
            AssignmentOp::NullCoalescenceEqual => BinOp::Coalesce,
            AssignmentOp::ModEqual => BinOp::Mod,
            AssignmentOp::DivEqual => BinOp::IDiv,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum UnaryOp {
    Not,
    BitwiseNot,
    Positive,
    Negative,
}

impl From<parse::UnaryOp> for UnaryOp {
    fn from(value: parse::UnaryOp) -> Self {
        match value {
            parse::UnaryOp::Negative => UnaryOp::Negative,
            parse::UnaryOp::Positive => UnaryOp::Positive,
            parse::UnaryOp::BitwiseNot => UnaryOp::BitwiseNot,
            parse::UnaryOp::Not => UnaryOp::Not,
        }
    }
}

#[mutants::skip]
impl std::fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOp::Not => f.pad("not"),
            UnaryOp::BitwiseNot => f.pad("bit_not"),
            UnaryOp::Positive => f.pad("positive"),
            UnaryOp::Negative => f.pad("negative"),
        }
    }
}
