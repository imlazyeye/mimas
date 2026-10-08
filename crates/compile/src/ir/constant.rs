use itertools::Itertools;
use parse::{Access, Expr, ExprKind, Literal};
use shared::StrId;
use solve::{
    ResolvedDeclKind,
    components::{AdtId, ConstValue, DecId},
};

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
    Dict(Vec<(StrId, Constant)>) = 6,
    Instance(AdtId, Vec<Constant>) = 7,
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
    Dict = 6,
    Instance = 7,
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
            Constant::Dict(fields) => {
                4 + fields.iter().map(|(_, v)| 5 + v.byte_len()).sum::<usize>()
            }
            Constant::Instance(_, fields) => {
                8 + fields.iter().map(|v| 1 + v.byte_len()).sum::<usize>()
            }
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
            Constant::Dict(fields) => {
                e.u32(fields.len() as u32);
                for (key, value) in fields {
                    e.u32(*key);
                    e.u8(constant_discriminant(value));
                    value.encode(e);
                }
            }
            Constant::Instance(adt, fields) => {
                e.u32(*adt);
                e.u32(fields.len() as u32);
                for value in fields {
                    e.u8(constant_discriminant(value));
                    value.encode(e);
                }
            }
        }
    }

    pub(crate) fn from_dec(ir: &mut Ir, dec: DecId) -> Self {
        match ir.resolutions.decs[dec].kind.clone() {
            ResolvedDeclKind::Constant(ConstValue::Literal(lit)) => Self::from_literal(ir, lit),
            ResolvedDeclKind::Constant(ConstValue::Expr(expr)) => Self::from_expr(ir, &expr),
            ResolvedDeclKind::Variant { layout, .. } => Self::Instance(layout, vec![]),
            ResolvedDeclKind::Adt(adt) => Self::Instance(adt, vec![]),
            _ => unreachable!("solver checked constant declarations"),
        }
    }

    fn from_expr(ir: &mut Ir, expr: &Expr) -> Self {
        match expr.kind() {
            ExprKind::Literal(lit) => Self::from_literal(ir, lit.clone()),
            ExprKind::Ident(_) | ExprKind::Access(Access::DoubleColon { .. }) => {
                Self::from_dec(ir, ir.node_dec(expr.id()))
            }
            ExprKind::Grouping(g) => Self::from_expr(ir, &g.inner),
            ExprKind::Call(call) => {
                let adt = *ir.resolutions.node_tys[&call.left.id()].as_adt().unwrap();
                let fields = call
                    .arguments
                    .iter()
                    .map(|arg| Self::from_expr(ir, &arg.value))
                    .collect();
                Self::Instance(adt, fields)
            }
            _ => {
                let lit = solve::utils::reduce(expr, &|e| {
                    let dec = ir.node_dec(e.id());
                    match &ir.resolutions.decs[dec].kind {
                        ResolvedDeclKind::Constant(ConstValue::Literal(lit)) => Some(lit.clone()),
                        _ => None,
                    }
                })
                .expect("solver checked constant arithmetic")
                .expect("solver checked constant expressions");
                Self::from_literal(ir, lit)
            }
        }
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
            Literal::Array(a) | Literal::Tuple(a) => {
                Self::Array(a.iter().map(|member| Self::from_expr(ir, member)).collect())
            }
            Literal::Dictionary(fields) => Self::Dict(
                fields
                    .iter()
                    .map(|(key, value)| (ir.intern_str(&key.lexeme), Self::from_expr(ir, value)))
                    .collect(),
            ),
            Literal::Struct(s) => {
                let adt = *ir.resolutions.node_tys[&s.name.id()].as_adt().unwrap();
                let order = ir.resolutions.adts[adt].fields.clone();
                let fields = order
                    .iter()
                    .map(|name| {
                        let (_, value) = s
                            .fields
                            .iter()
                            .find(|(key, _)| key.to_string() == *name)
                            .unwrap();
                        Self::from_expr(ir, value)
                    })
                    .collect();
                Self::Instance(adt, fields)
            }
        }
    }

    /// Emit a constant's value with fresh containers. Reads that only inspect it use
    /// `Ir::shared_for`.
    pub(crate) fn emit_const_dec(ir: &mut Ir, dec: DecId) -> Option<crate::InstId> {
        let constant = Self::from_dec(ir, dec);
        Some(ir.current().constant(constant))
    }

    /// Returns the [CachedConstant] version of this const, if available. These are Hash friendly.
    pub(crate) fn cached(&self) -> Option<CachedConstant> {
        match self {
            Constant::Bool(b) => Some(CachedConstant::Bool(*b)),
            Constant::Int(i) => Some(CachedConstant::Int(*i)),
            Constant::Str(str_id) => Some(CachedConstant::Str(*str_id)),
            Constant::Null => Some(CachedConstant::Null),
            Constant::Float(f) => Some(CachedConstant::Float(f.to_bits())),
            Constant::Array(_) | Constant::Dict(_) | Constant::Instance(_, _) => None,
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
            Constant::Array(_) | Constant::Dict(_) | Constant::Instance(_, _) => self.to_string(),
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
            Constant::Dict(fields) => f.pad(&format!(
                "~{{ {} }}",
                fields
                    .iter()
                    .map(|(key, value)| format!("str {} = {value}", key.index()))
                    .join(", ")
            )),
            Constant::Instance(adt, fields) => f.pad(&format!(
                "adt {}({})",
                adt.index(),
                fields.iter().join(", ")
            )),
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
