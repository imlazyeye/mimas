use itertools::Itertools;
use parse::{Expr, ExprKind, Literal};
use shared::StrId;
use solve::{components::DecId, utils::reduce_simple};

use super::{Inst, Ir, IrDisplay};

// the discriminants here are the on-wire tag -- keep them in sync with ConstantCode.
#[repr(u8)]
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Bool(bool) = 0,
    Int(i64) = 1,
    Float(f64) = 2,
    Str(StrId) = 3,
    Array(Vec<Constant>) = 4,
    Null = 5,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ConstantCode {
    Bool = 0,
    Int = 1,
    Float = 2,
    Str = 3,
    Array = 4,
    Null = 5,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CachedConstant {
    Bool(bool),
    Int(i64),
    Float(u64),
    Str(StrId),
    Null,
}

impl CachedConstant {
    /// Back to a full [Constant] so the prologue can emit its load.
    pub(crate) fn to_constant(&self) -> Constant {
        match self {
            CachedConstant::Bool(b) => Constant::Bool(*b),
            CachedConstant::Int(i) => Constant::Int(*i),
            CachedConstant::Float(bits) => Constant::Float(f64::from_bits(*bits)),
            CachedConstant::Str(s) => Constant::Str(*s),
            CachedConstant::Null => Constant::Null,
        }
    }
}

#[inline(always)]
pub fn constant_discriminant(c: &Constant) -> u8 {
    unsafe { *(c as *const Constant as *const u8) }
}

impl Constant {
    pub fn byte_len(&self) -> usize {
        match self {
            Constant::Bool(_) => 1,
            Constant::Int(_) | Constant::Float(_) => 8,
            Constant::Str(_) => 4,
            Constant::Array(a) => 4 + a.iter().map(|v| 1 + v.byte_len()).sum::<usize>(),
            Constant::Null => 0,
        }
    }

    pub(crate) fn encode(&self, e: &mut crate::Encoder) {
        match self {
            Constant::Bool(b) => e.u8(*b as u8),
            Constant::Int(i) => e.i64(*i),
            Constant::Float(f) => e.f64(*f),
            Constant::Str(str_id) => e.u32(*str_id),
            Constant::Array(a) => {
                e.u32(a.len() as u32);
                for v in a {
                    e.u8(constant_discriminant(v));
                    v.encode(e);
                }
            }
            Constant::Null => {}
        }
    }

    pub(crate) fn from_dec(ir: &mut Ir, dec: DecId) -> Self {
        let solve::ResolvedDeclKind::Constant(lit) = &ir.resolutions.decs[dec].kind else {
            unreachable!("Constant::from_dec called on non-constant dec {dec:?}");
        };
        let lit = lit.clone();
        Self::from_literal(ir, lit)
    }

    pub(crate) fn from_literal(ir: &mut Ir, lit: Literal) -> Self {
        match lit {
            Literal::True => Self::Bool(true),
            Literal::False => Self::Bool(false),
            Literal::Null | Literal::Unit => Self::Null,
            Literal::Int(i) => Self::Int(i),
            Literal::Float(f) => Self::Float(f),
            Literal::Hex(h) => Self::Int(i64::from_str_radix(&h, 16).unwrap()),
            Literal::String(s) => Self::Str(ir.intern_str(&s)),
            // arrays and tuples share the same runtime shape -- both lower to a `NewArray` + pushes
            // at use sites today (see `Literal::emit`), so the const form mirrors that.
            Literal::Array(a) | Literal::Tuple(a) => {
                Self::Array(a.iter().map(|e| Self::from_expr(ir, e)).collect())
            }
            // dicts and structs allocate distinct runtime types; the `const` site can't inline
            // them as a bytecode Constant. callers should use `Self::emit_const_dec` (which
            // routes through `Literal::emit`) instead of trying to fold to a `Constant`.
            Literal::Dictionary(_) | Literal::Struct(_) => unreachable!(
                "Constant::from_literal: dict/struct have no scalar constant form -- use emit_const_dec",
            ),
        }
    }

    /// Emit a use-site reference to a const dec. Scalar/array/tuple literals lower to a
    /// `LoadConst` inst; dict/struct literals lower to a fresh runtime allocation at the
    /// use site (same `NewDict`/`NewInstance` path as inline literals would take). `id` is
    /// the use-site's node id, needed by `Literal::Struct` to resolve the ADT type.
    pub(crate) fn emit_const_dec(
        ir: &mut Ir,
        dec: DecId,
        id: parse::NodeId,
    ) -> Option<crate::InstId> {
        let solve::ResolvedDeclKind::Constant(lit) = &ir.resolutions.decs[dec].kind else {
            unreachable!("emit_const_dec called on non-constant dec {dec:?}");
        };
        let lit = lit.clone();
        match &lit {
            Literal::Dictionary(_) | Literal::Struct(_) => {
                use super::Emit;
                lit.emit(id, ir)
            }
            _ => {
                let constant = Self::from_literal(ir, lit);
                Some(ir.current().constant(constant))
            }
        }
    }

    // inside a const array, each element is: ident-to-const, nested array, or a reducible expr.
    // safe to unwrap both layers: solve has already verified every element folds (`is_const_expr`)
    // and would have surfaced any divide-by-zero / overflow as a diagnostic before IR runs.
    fn from_expr(ir: &mut Ir, expr: &Expr) -> Self {
        match expr.kind() {
            ExprKind::Ident(_) => Self::from_dec(ir, ir.node_dec(expr.id())),
            ExprKind::Literal(Literal::Array(inner)) => {
                Self::Array(inner.iter().map(|e| Self::from_expr(ir, e)).collect())
            }
            _ => Self::from_literal(
                ir,
                // solve already vetted every element; reduce_simple is source-free and
                // either succeeds or the expr is malformed (which would've failed type-check).
                // both unwraps lean on that.
                reduce_simple(expr)
                    .expect("solve verified every const array element folds")
                    .expect("solve verified every const array element folds to Some"),
            ),
        }
    }

    /// Returns the [CachedConstant] version of this const, if available. These are Hash friendly.
    pub(crate) fn cached(&self) -> Option<CachedConstant> {
        match self {
            Constant::Bool(b) => Some(CachedConstant::Bool(*b)),
            Constant::Int(i) => Some(CachedConstant::Int(*i)),
            Constant::Str(str_id) => Some(CachedConstant::Str(*str_id)),
            Constant::Null => Some(CachedConstant::Null),
            Constant::Float(f) => Some(CachedConstant::Float(f.to_bits())),
            Constant::Array(_) => None,
        }
    }
}

#[mutants::skip]
impl IrDisplay for Constant {
    fn ir_display(&self, ir: &Ir) -> String {
        match self {
            Constant::Bool(b) => format!("{b:?}"),
            Constant::Int(i) => i.to_string(),
            Constant::Float(float) => float.to_string(),
            Constant::Str(str_id) => format!("\"{}\"", ir.str(*str_id)),
            Constant::Array(_) => self.to_string(),
            Constant::Null => "null".into(),
        }
    }
}

#[mutants::skip]
impl std::fmt::Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constant::Bool(b) => f.pad(&format!("{b}")),
            Constant::Int(i) => f.pad(&format!("{i}")),
            Constant::Float(v) => f.pad(&format!("{v}")),
            Constant::Str(str_id) => f.pad(&format!("str {}", str_id.index())),
            Constant::Array(a) => {
                f.pad(&format!("[{}]", a.iter().map(|v| v.to_string()).join(", ")))
            }
            Constant::Null => f.pad("null"),
        }
    }
}

// todo: could macro the numericals
impl From<Constant> for Inst {
    fn from(value: Constant) -> Self {
        Self::Constant(value)
    }
}

impl From<usize> for Constant {
    fn from(value: usize) -> Self {
        Self::Int(value as i64)
    }
}

impl From<i64> for Constant {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<i32> for Constant {
    fn from(value: i32) -> Self {
        Self::Int(value as i64)
    }
}

impl From<f64> for Constant {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<bool> for Constant {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
