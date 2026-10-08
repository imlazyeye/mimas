#![allow(non_snake_case)]

use parse::expr::{EqualityOp::*, EvaluationOp::*, Literal::*, UnaryOp::*, *};
use shared::{Located, Location};

use crate::{Error, Solver, errors};

/// Raw reduction error -- carries just the location of the offending expression. The actual
/// miette `Diagnostic` (with `NamedSource`) is materialized at the boundary by
/// [`ReduceError::into_diag`], so this module stays free of any source-attachment plumbing.
#[derive(Debug)]
pub enum ReduceError {
    InvalidHex(Location),
    DivideByZero(Location),
    InvalidShift(Location),
    Overflow(Location),
}

pub type ReduceResult<T> = std::result::Result<T, ReduceError>;

impl ReduceError {
    /// Look up the source through `solver` and produce a full miette diagnostic.
    pub fn into_diag(self, solver: &Solver) -> Error {
        match self {
            Self::InvalidHex(loc) => errors::InvalidHex {
                src: solver.src(loc),
                at: loc.into(),
            }
            .into(),
            Self::DivideByZero(loc) => errors::DivideByZero {
                src: solver.src(loc),
                at: loc.into(),
            }
            .into(),
            Self::InvalidShift(loc) => errors::InvalidShift {
                src: solver.src(loc),
                at: loc.into(),
            }
            .into(),
            Self::Overflow(loc) => errors::ConstOverflow {
                src: solver.src(loc),
                at: loc.into(),
            }
            .into(),
        }
    }
}

/// Constant-folds an expression. The single source of truth for reduction shape -- `resolve`
/// lets callers plug in their own meaning for name leaves (e.g. consult a const table).
/// Restricted to leaves and arithmetic-style composites (no `Block`/`If`) because that's all
/// const-position rhs's need; lifting that restriction means revisiting `const_ready` too.
pub fn reduce<F: Fn(&Expr) -> Option<Literal>>(
    expr: &Expr,
    resolve: &F,
) -> ReduceResult<Option<Literal>> {
    let lit = |e| reduce(e, resolve);
    match expr.kind() {
        ExprKind::Ident(_) | ExprKind::Access(Access::DoubleColon { .. }) => Ok(resolve(expr)),
        ExprKind::Literal(l) => reduce_literal(l, expr.location(), resolve),
        ExprKind::Grouping(g) => lit(&g.inner),
        ExprKind::Unary(u) => {
            let Some(right) = lit(&u.right)? else {
                return Ok(None);
            };
            Ok(match (&u.op, right) {
                (Not, True) => Some(False),
                (Not, False) => Some(True),
                (Positive, Int(v)) => Some(Int(v
                    .checked_abs()
                    .ok_or(ReduceError::Overflow(expr.location()))?)),
                (Positive, Float(v)) => Some(Float(v)),
                (Negative, Int(v)) => Some(Int(v
                    .checked_neg()
                    .ok_or(ReduceError::Overflow(expr.location()))?)),
                (Negative, Float(v)) => Some(Float(std::ops::Neg::neg(v))),
                (BitwiseNot, Int(v)) => Some(Int(std::ops::Not::not(v))),
                _ => None,
            })
        }
        ExprKind::Evaluation(e) => {
            let (Some(l), Some(r)) = (lit(&e.left)?, lit(&e.right)?) else {
                return Ok(None);
            };
            evaluate(l, e.op, r, expr.location())
        }
        ExprKind::Equality(e) => {
            let (Some(l), Some(r)) = (lit(&e.left)?, lit(&e.right)?) else {
                return Ok(None);
            };
            Ok(compare(l, e.op, r))
        }
        ExprKind::Logical(l) => {
            let Some(lhs) = lit(&l.left)? else {
                return Ok(None);
            };
            if matches!(
                (&lhs, l.op),
                (False, LogicalOp::And) | (True, LogicalOp::Or)
            ) {
                return Ok(Some(lhs));
            }
            let Some(rhs) = lit(&l.right)? else {
                return Ok(None);
            };
            Ok(match (lhs, rhs) {
                (True, True) => Some(True),
                (False, False) => Some(False),
                (True, False) | (False, True) => match l.op {
                    LogicalOp::And => Some(False),
                    LogicalOp::Or => Some(True),
                },
                _ => None,
            })
        }
        _ => Ok(None),
    }
}

/// Reduce with no Ident-resolution context. Convenience wrapper for callers that don't have
/// a const table (tests).
pub fn reduce_simple(expr: &Expr) -> ReduceResult<Option<Literal>> {
    reduce(expr, &|_| None)
}

fn reduce_literal<F: Fn(&Expr) -> Option<Literal>>(
    lit: &Literal,
    location: Location,
    resolve: &F,
) -> ReduceResult<Option<Literal>> {
    match lit {
        True | False | Null | Unit | String(_) | Int(_) | Float(_) => Ok(Some(lit.clone())),
        Hex(s) => Ok(Some(Int(i64::from_str_radix(
            s.trim_start_matches("0x"),
            16,
        )
        .map_err(|_| ReduceError::InvalidHex(location))?))),
        // an array or tuple folds to one of its folded members
        Array(a) | Tuple(a) => {
            let mut members = Vec::with_capacity(a.len());
            for e in a {
                let Some(member) = reduce(e, resolve)? else {
                    return Ok(None);
                };
                members.push(Expr::new(member.into(), e.location()));
            }
            Ok(Some(match lit {
                Array(_) => Array(members),
                _ => Tuple(members),
            }))
        }
        // these have no folded form (see `ConstValue::Expr`)
        Dictionary(_) | Struct(_) => Ok(None),
    }
}

fn evaluate(
    lhs: Literal,
    op: EvaluationOp,
    rhs: Literal,
    location: Location,
) -> ReduceResult<Option<Literal>> {
    fn math_floats(a: f64, op: EvaluationOp, b: f64) -> Option<f64> {
        match op {
            Plus => Some(a + b),
            Minus => Some(a - b),
            Divide => Some(a / b),
            Multiply => Some(a * b),
            Div => Some((a / b).floor()),
            Modulo => Some(a % b),
            _ => None,
        }
    }

    Ok(match (lhs, rhs) {
        (String(str_a), String(str_b)) => {
            let mut new_str = str_a;
            new_str.push_str(&str_b);
            Some(Literal::String(new_str))
        }

        (Int(a), Int(b)) => match op {
            Plus => Some(Int(a
                .checked_add(b)
                .ok_or(ReduceError::Overflow(location))?)),
            Minus => Some(Int(a
                .checked_sub(b)
                .ok_or(ReduceError::Overflow(location))?)),
            Divide => Some(Float(a as f64 / b as f64)),
            Multiply => Some(Int(a
                .checked_mul(b)
                .ok_or(ReduceError::Overflow(location))?)),
            Div => {
                if b == 0 {
                    Err(ReduceError::DivideByZero(location))?
                } else {
                    Some(Int(a
                        .checked_div(b)
                        .ok_or(ReduceError::Overflow(location))?))
                }
            }
            Modulo => {
                if b == 0 {
                    Err(ReduceError::DivideByZero(location))?
                } else {
                    Some(Int(a
                        .checked_rem(b)
                        .ok_or(ReduceError::Overflow(location))?))
                }
            }
            And => Some(Int(a & b)),
            Or => Some(Int(a | b)),
            Xor => Some(Int(a ^ b)),
            BitShiftLeft => Some(Int(a
                .checked_shl(
                    b.try_into()
                        .map_err(|_| ReduceError::InvalidShift(location))?,
                )
                .ok_or(ReduceError::InvalidShift(location))?)),
            BitShiftRight => Some(Int(a
                .checked_shr(
                    b.try_into()
                        .map_err(|_| ReduceError::InvalidShift(location))?,
                )
                .ok_or(ReduceError::InvalidShift(location))?)),
        },

        (Float(float_a), Float(float_b)) => math_floats(float_a, op, float_b).map(Float),
        (Int(int), Float(float)) => math_floats(int as f64, op, float).map(Float),
        (Float(float), Int(int)) => math_floats(float, op, int as f64).map(Float),

        _ => None,
    })
}

fn compare(lhs: Literal, op: EqualityOp, rhs: Literal) -> Option<Literal> {
    fn ordered<T: PartialOrd>(a: T, op: EqualityOp, b: T) -> Literal {
        match op {
            Greater if a > b => True,
            Greater => False,
            GreaterOrEqual if a >= b => True,
            GreaterOrEqual => False,
            Less if a < b => True,
            Less => False,
            LessOrEqual if a <= b => True,
            LessOrEqual => False,
            _ => unreachable!(),
        }
    }

    let (lhs, rhs) = match (lhs, rhs) {
        (Int(int), Float(float)) => (Float(int as f64), Float(float)),
        (Float(float), Int(int)) => (Float(float), Float(int as f64)),
        other => other,
    };

    match op {
        Equal if lhs == rhs => Some(True),
        Equal if lhs != rhs => Some(False),
        NotEqual if lhs == rhs => Some(False),
        NotEqual if lhs != rhs => Some(True),
        _ => match (lhs, rhs) {
            (Int(a), Int(b)) => Some(ordered(a, op, b)),
            (Float(a), Float(b)) => Some(ordered(a, op, b)),
            (String(a), String(b)) => Some(ordered(a, op, b)),
            _ => None,
        },
    }
}
