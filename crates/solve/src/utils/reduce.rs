#![allow(non_snake_case)]

use parse::expr::{EqualityOp::*, EvaluationOp::*, Literal::*, UnaryOp::*, *};
use shared::{Located, Location};

use crate::{Error, Solver, errors};

/// Raw reduction error -- carries just the location of the offending expression. The actual
/// miette `Diagnostic` (with `NamedSource`) is materialized at the boundary by
/// [`ReduceError::into_diag`], so this module stays free of any source-attachment plumbing.
/// Callers that have proven no errors can fire (e.g. compile's IR lowering) just `.expect()`.
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
/// lets callers plug in their own meaning for `Ident` leaves (e.g. consult a const table).
/// Restricted to leaves and arithmetic-style composites (no `Block`/`If`) because that's all
/// const-position rhs's need; lifting that restriction means revisiting `is_const_expr` too.
pub fn reduce<F: Fn(&Expr) -> Option<Literal>>(
    expr: &Expr,
    resolve: &F,
) -> ReduceResult<Option<Literal>> {
    let lit = |e| reduce(e, resolve);
    match expr.kind() {
        ExprKind::Ident(_) => Ok(resolve(expr)),
        ExprKind::Literal(l) => reduce_literal(l, expr.location(), resolve),
        ExprKind::Grouping(g) => lit(&g.inner),
        ExprKind::Unary(u) => {
            let Some(right) = lit(&u.right)? else {
                return Ok(None);
            };
            Ok(match (&u.op, right) {
                (Not, True) => Some(False),
                (Not, False) => Some(True),
                (Positive, Int(v)) => Some(Int(i64::abs(v))),
                (Positive, Float(v)) => Some(Float(f64::abs(v))),
                (Negative, Int(v)) => Some(Int(std::ops::Neg::neg(v))),
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
            let (Some(lhs), Some(rhs)) = (lit(&l.left)?, lit(&l.right)?) else {
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
/// a const table (tests, compile's IR lowering of already-vetted const arrays).
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
        Array(a) | Tuple(a) => {
            for e in a.iter() {
                if reduce(e, resolve)?.is_none() {
                    return Ok(None);
                }
            }
            Ok(Some(lit.clone()))
        }
        Dictionary(fields) => {
            for (_, e) in fields.iter() {
                if reduce(e, resolve)?.is_none() {
                    return Ok(None);
                }
            }
            Ok(Some(lit.clone()))
        }
        Struct(s) => {
            for (_, e) in s.fields.iter() {
                if reduce(e, resolve)?.is_none() {
                    return Ok(None);
                }
            }
            Ok(Some(lit.clone()))
        }
    }
}

fn evaluate(
    lhs: Literal,
    op: EvaluationOp,
    rhs: Literal,
    location: Location,
) -> ReduceResult<Option<Literal>> {
    fn math_floats(
        a: f64,
        op: EvaluationOp,
        b: f64,
        location: Location,
    ) -> ReduceResult<Option<f64>> {
        Ok(match op {
            Plus => Some(a + b),
            Minus => Some(a - b),
            Divide => {
                if b == 0.0 {
                    Err(ReduceError::DivideByZero(location))?
                } else {
                    Some(a / b)
                }
            }
            Multiply => Some(a * b),
            _ => None,
        })
    }

    Ok(match (lhs, rhs) {
        (String(str_a), String(str_b)) => {
            let mut new_str = str_a;
            new_str.push_str(&str_b);
            Some(Literal::String(new_str))
        }

        (Int(a), Int(b)) => match op {
            Plus => Some(Int(a.wrapping_add(b))),
            Minus => Some(Int(a.wrapping_sub(b))),
            Divide => Some(Float(a as f64 / b as f64)),
            Multiply => Some(Int(a.wrapping_mul(b))),
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
            BitShiftLeft => Some(Int(a.wrapping_shl(
                b.try_into()
                    .map_err(|_| ReduceError::InvalidShift(location))?,
            ))),
            BitShiftRight => Some(Int(a.wrapping_shr(
                b.try_into()
                    .map_err(|_| ReduceError::InvalidShift(location))?,
            ))),
        },

        (Float(float_a), Float(float_b)) => math_floats(float_a, op, float_b, location)?.map(Float),
        (Int(int), Float(float)) => math_floats(int as f64, op, float, location)?.map(Float),
        (Float(float), Int(int)) => math_floats(float, op, int as f64, location)?.map(Float),

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
            (Int(a), Int(b)) => Some(ordered(a as f64, op, b as f64)),
            (Float(a), Float(b)) => Some(ordered(a, op, b)),
            (String(a), String(b)) => Some(ordered(a, op, b)),
            _ => None,
        },
    }
}
