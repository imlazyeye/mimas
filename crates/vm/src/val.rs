use std::{collections::HashMap, sync::Arc};

use compile::{BinFault, BinOp, Scalar, UnaryOp};
use gc_arena::{Collect, Gc, RefLock};
use shared::BodyId;
use smallvec::SmallVec;

use crate::{RtErr, RtResult, heap::Ctx};

pub type SharedStr = Arc<str>;

#[derive(Copy, Clone, Default, Collect, Debug)]
#[collect(no_drop)]
pub enum Val<'gc> {
    #[default]
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Fn(#[collect(require_static)] BodyId),
    Str(Str<'gc>),
    Array(Array<'gc>),
    Dict(Dict<'gc>),
    Instance(Instance<'gc>),
    Closure(Closure<'gc>),
    Raised(Str<'gc>),
}

impl<'gc> PartialEq for Val<'gc> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Val::Null, Val::Null) => true,
            (Val::Bool(a), Val::Bool(b)) => a == b,
            (Val::Int(a), Val::Int(b)) => a == b,
            (Val::Float(a), Val::Float(b)) => a == b,
            (Val::Fn(a), Val::Fn(b)) => a == b,
            (Val::Str(a), Val::Str(b)) => a == b,
            (Val::Array(a), Val::Array(b)) => {
                Gc::ptr_eq(a.0, b.0) || *a.0.borrow() == *b.0.borrow()
            }
            (Val::Dict(a), Val::Dict(b)) => Gc::ptr_eq(a.0, b.0) || *a.0.borrow() == *b.0.borrow(),
            (Val::Instance(a), Val::Instance(b)) => {
                Gc::ptr_eq(a.0, b.0) || {
                    let (a, b) = (a.0.borrow(), b.0.borrow());
                    a.struct_id == b.struct_id && a.fields.as_slice() == b.fields.as_slice()
                }
            }
            (Val::Closure(a), Val::Closure(b)) => Gc::ptr_eq(a.0, b.0),
            (Val::Raised(a), Val::Raised(b)) => a == b,
            _ => false,
        }
    }
}

impl<'gc> Val<'gc> {
    #[inline]
    pub fn as_int(self) -> Option<i64> {
        if let Val::Int(n) = self {
            Some(n)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_float(self) -> Option<f64> {
        if let Val::Float(n) = self {
            Some(n)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_bool(self) -> Option<bool> {
        if let Val::Bool(b) = self {
            Some(b)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_str(self) -> Option<Str<'gc>> {
        if let Val::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }
    #[inline]
    pub fn as_array(self) -> Option<Array<'gc>> {
        if let Val::Array(a) = self {
            Some(a)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_dict(self) -> Option<Dict<'gc>> {
        if let Val::Dict(d) = self {
            Some(d)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_instance(self) -> Option<Instance<'gc>> {
        if let Val::Instance(i) = self {
            Some(i)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_closure(self) -> Option<Closure<'gc>> {
        if let Val::Closure(c) = self {
            Some(c)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_fn(self) -> Option<BodyId> {
        if let Val::Fn(b) = self { Some(b) } else { None }
    }

    #[inline]
    pub fn as_raised(self) -> Option<Str<'gc>> {
        if let Val::Raised(s) = self {
            Some(s)
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, Collect, Debug)]
#[collect(no_drop)]
pub struct Str<'gc>(pub Gc<'gc, SharedStr>);

#[derive(Copy, Clone, Collect, Debug)]
#[collect(no_drop)]
pub struct Array<'gc>(pub Gc<'gc, RefLock<Vec<Val<'gc>>>>);

#[derive(Copy, Clone, Collect, Debug)]
#[collect(no_drop)]
pub struct Dict<'gc>(pub Gc<'gc, RefLock<DictMap<'gc>>>);

// generic over the value slot so natives can take a type-safe `DictMap<'gc, anon::T<'gc>>` view
// (see `anon::as_dict_mut`); gc storage is always the `V = Val` default.
#[derive(Collect, Debug)]
#[collect(no_drop)]
pub struct DictMap<'gc, V = Val<'gc>> {
    // entries holds (key, value) in insertion order for stable positional iteration; index maps
    // each key to its slot in entries. invariant: index[k] == position of k in entries.
    entries: Vec<(Str<'gc>, V)>,
    index: HashMap<Str<'gc>, usize>,
}

impl<'gc, V> Default for DictMap<'gc, V> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            index: HashMap::new(),
        }
    }
}

impl<'gc, V> DictMap<'gc, V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, key: &Str<'gc>) -> Option<&V> {
        self.index.get(key).map(|&i| &self.entries[i].1)
    }

    pub fn contains_key(&self, key: &Str<'gc>) -> bool {
        self.index.contains_key(key)
    }

    pub fn insert(&mut self, key: Str<'gc>, val: V) -> Option<V> {
        if let Some(&i) = self.index.get(&key) {
            Some(std::mem::replace(&mut self.entries[i].1, val))
        } else {
            self.index.insert(key, self.entries.len());
            self.entries.push((key, val));
            None
        }
    }

    pub fn remove(&mut self, key: &Str<'gc>) -> Option<V> {
        let i = self.index.remove(key)?;
        let (_, val) = self.entries.remove(i);
        for (k, _) in &self.entries[i..] {
            *self.index.get_mut(k).unwrap() -= 1;
        }
        Some(val)
    }

    pub fn entry_at(&self, i: usize) -> (Str<'gc>, V)
    where
        V: Copy,
    {
        self.entries[i]
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Str<'gc>, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }
}

impl<'gc, V: PartialEq> PartialEq for DictMap<'gc, V> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|(k, v)| other.get(k) == Some(v))
    }
}

impl<'gc, V> FromIterator<(Str<'gc>, V)> for DictMap<'gc, V> {
    fn from_iter<I: IntoIterator<Item = (Str<'gc>, V)>>(iter: I) -> Self {
        let mut map = DictMap::new();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}

#[derive(Copy, Clone, Collect, Debug)]
#[collect(no_drop)]
pub struct Instance<'gc>(pub Gc<'gc, RefLock<InstanceData<'gc>>>);

#[derive(Collect, Debug)]
#[collect(no_drop)]
pub struct InstanceData<'gc> {
    #[collect(require_static)]
    pub struct_id: u32,
    pub fields: Fields<'gc>,
}

/// The number of fields an ADT can have where we will inline its fields instead of spilling into a
/// newly allocated Vec.
///
/// This could/should eventually be configurable by the user. Depending on your project, you could
/// probably target what the exact cutoff is for your hot-path ADTs.
pub const INLINE_FIELDS: usize = 4;

#[derive(Clone, Collect, Debug)]
#[collect(no_drop)]
pub enum Fields<'gc> {
    Inline {
        #[collect(require_static)]
        len: u8,
        data: [Val<'gc>; INLINE_FIELDS],
    },
    Spilled(Vec<Val<'gc>>),
}

impl<'gc> Fields<'gc> {
    pub fn new(values: Vec<Val<'gc>>) -> Self {
        if values.len() <= INLINE_FIELDS {
            let len = values.len() as u8;
            let mut data = [Val::Null; INLINE_FIELDS];
            for (slot, v) in values.into_iter().enumerate() {
                data[slot] = v;
            }
            Fields::Inline { len, data }
        } else {
            Fields::Spilled(values)
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Fields::Inline { len, .. } => *len as usize,
            Fields::Spilled(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_slice(&self) -> &[Val<'gc>] {
        match self {
            Fields::Inline { len, data } => &data[..*len as usize],
            Fields::Spilled(v) => v,
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [Val<'gc>] {
        match self {
            Fields::Inline { len, data } => &mut data[..*len as usize],
            Fields::Spilled(v) => v,
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Val<'gc>> {
        self.as_slice().iter()
    }
}

impl<'gc> std::ops::Index<usize> for Fields<'gc> {
    type Output = Val<'gc>;
    fn index(&self, i: usize) -> &Val<'gc> {
        &self.as_slice()[i]
    }
}

impl<'gc> std::ops::IndexMut<usize> for Fields<'gc> {
    fn index_mut(&mut self, i: usize) -> &mut Val<'gc> {
        &mut self.as_mut_slice()[i]
    }
}

impl<'gc> IntoIterator for Fields<'gc> {
    type Item = Val<'gc>;
    type IntoIter = smallvec::IntoIter<[Val<'gc>; INLINE_FIELDS]>;
    fn into_iter(self) -> Self::IntoIter {
        let values: SmallVec<[Val<'gc>; INLINE_FIELDS]> = match self {
            Fields::Inline { len, data } => SmallVec::from_slice(&data[..len as usize]),
            Fields::Spilled(v) => SmallVec::from_vec(v),
        };
        values.into_iter()
    }
}

#[derive(Copy, Clone, Collect, Debug)]
#[collect(no_drop)]
pub struct Closure<'gc>(pub Gc<'gc, ClosureData<'gc>>);

#[derive(Collect, Debug)]
#[collect(no_drop)]
pub struct ClosureData<'gc> {
    #[collect(require_static)]
    pub function: BodyId,
    pub captures: Vec<Val<'gc>>,
}

impl<'gc> Str<'gc> {
    pub fn as_str(self) -> &'gc str {
        Gc::as_ref(self.0).as_ref()
    }
}

impl<'gc> PartialEq for Str<'gc> {
    fn eq(&self, other: &Self) -> bool {
        Gc::ptr_eq(self.0, other.0)
    }
}
impl<'gc> Eq for Str<'gc> {}

impl<'gc> std::hash::Hash for Str<'gc> {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        Gc::as_ptr(self.0).hash(h);
    }
}

/// A gc-free snapshot of a [`Val<'gc>`] tree. Used for tests.
#[derive(Debug, Clone, PartialEq)]
pub enum Captured {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Array(Vec<Captured>),
    Dict(Vec<(String, Captured)>),
    Instance(Vec<Captured>),
    Fn(BodyId),
    Raised(String),
    /// Catch-all for Fn / Closure -- these aren't expected to appear as test outputs, but
    /// if they do, comparing against `Other` will fail loudly rather than panic.
    Other,
}

impl std::fmt::Display for Captured {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Captured::Null => f.write_str("null"),
            Captured::Bool(b) => write!(f, "{b}"),
            Captured::Int(i) => write!(f, "{i}"),
            Captured::Float(n) => write!(f, "{n}"),
            Captured::Str(s) => write!(f, "{s:?}"),
            Captured::Array(items) => {
                f.write_str("[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{v}")?;
                }
                f.write_str("]")
            }
            Captured::Dict(entries) => {
                f.write_str("~{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{k} = {v}")?;
                }
                f.write_str("}")
            }
            Captured::Instance(fields) => {
                f.write_str("{ ")?;
                for (i, v) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{v}")?;
                }
                f.write_str(" }")
            }
            Captured::Fn(body_id) => write!(f, "fn({})", body_id.index()),
            Captured::Raised(s) => write!(f, "raised({s:?})"),
            Captured::Other => f.write_str("<other>"),
        }
    }
}

impl<'gc> Val<'gc> {
    /// Recursively snapshot `self` into a gc-free [`Captured`] tree. Must be called
    /// inside the arena's `mutate` scope; the resulting `Captured` can safely escape.
    pub fn capture(self) -> Captured {
        match self {
            Val::Null => Captured::Null,
            Val::Bool(b) => Captured::Bool(b),
            Val::Int(i) => Captured::Int(i),
            Val::Float(f) => Captured::Float(f),
            Val::Str(s) => Captured::Str(s.as_str().to_string()),
            Val::Array(a) => {
                Captured::Array(a.0.borrow().iter().copied().map(Val::capture).collect())
            }
            Val::Dict(d) => Captured::Dict(
                d.0.borrow()
                    .iter()
                    .map(|(k, v)| (k.as_str().to_string(), v.capture()))
                    .collect(),
            ),
            Val::Instance(inst) => Captured::Instance(
                inst.0
                    .borrow()
                    .fields
                    .iter()
                    .copied()
                    .map(Val::capture)
                    .collect(),
            ),
            Val::Fn(body_id) => Captured::Fn(body_id),
            Val::Raised(s) => Captured::Raised(s.as_str().to_string()),
            Val::Closure(_) => Captured::Other,
        }
    }
}

pub fn bin<'gc>(this: Val<'gc>, ctx: Ctx<'gc>, other: Val<'gc>, op: BinOp) -> RtResult<Val<'gc>> {
    // inner helpers rebuild the operand `Val`s from their primitives so the
    // `InvalidBinOperands` error gets concrete context. for coerced operands
    // (e.g. Float * Int, both lowered to f64) the rebuilt vals reflect the
    // coerced form -- the lossless `this`/`other` are at the outer match.
    fn bool_bin<'gc>(op: BinOp, a: bool, b: bool) -> RtResult<Val<'gc>> {
        Ok(match op {
            BinOp::And | BinOp::BitAnd => Val::Bool(a && b),
            BinOp::Or | BinOp::BitOr => Val::Bool(a || b),
            BinOp::Xor | BinOp::BitXor => Val::Bool(a ^ b),
            BinOp::Identity => Val::Bool(a == b),
            BinOp::NotEqual => Val::Bool(a != b),
            _ => Err(RtErr::invalid_bin(Val::Bool(a), op, Val::Bool(b)))?,
        })
    }

    fn scalar_val<'gc>(s: Scalar) -> Val<'gc> {
        match s {
            Scalar::Int(i) => Val::Int(i),
            Scalar::Bool(b) => Val::Bool(b),
            Scalar::Float(f) => Val::Float(f),
        }
    }

    fn float_bin<'gc>(op: BinOp, a: f64, b: f64) -> RtResult<Val<'gc>> {
        op.eval_float(a, b)
            .map(scalar_val)
            .map_err(|_| RtErr::invalid_bin(Val::Float(a), op, Val::Float(b)))
    }

    fn int_bin<'gc>(op: BinOp, a: i64, b: i64) -> RtResult<Val<'gc>> {
        op.eval_int(a, b)
            .map(scalar_val)
            .map_err(|fault| match fault {
                BinFault::Overflow => RtErr::IntegerOverflow,
                BinFault::DivByZero => RtErr::DivByZero,
                BinFault::ModByZero => RtErr::ModByZero,
                BinFault::InvalidShift => RtErr::InvalidShift,
                BinFault::Type => RtErr::invalid_bin(Val::Int(a), op, Val::Int(b)),
            })
    }

    Ok(match (op, this, other) {
        (BinOp::Add, Val::Str(a), Val::Str(b)) => {
            let combined = format!("{}{}", a.as_str(), b.as_str());
            Val::Str(ctx.intern(&combined))
        }
        (BinOp::Coalesce, Val::Null, other) => other,
        (BinOp::Coalesce, this, _) => this,
        (BinOp::NotEqual, left, right) => Val::Bool(left != right),
        (op, Val::Bool(a), Val::Bool(b)) => bool_bin(op, a, b)?,
        (op, Val::Float(a), Val::Float(b)) => float_bin(op, a, b)?,
        (op, Val::Float(a), Val::Int(b)) => float_bin(op, a, b as f64)?,
        (op, Val::Int(a), Val::Float(b)) => float_bin(op, a as f64, b)?,
        (op, Val::Int(a), Val::Int(b)) => int_bin(op, a, b)?,
        (op, Val::Array(a), Val::Array(b)) => {
            let n = a.0.borrow().len();
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                // re-borrow each iteration so the recursive bin call can allocate freely.
                let l = a.0.borrow()[i];
                let r = b.0.borrow()[i];
                out.push(bin(l, ctx, r, op)?);
            }
            Val::Array(ctx.new_array(out))
        }
        (op, Val::Array(a), s @ (Val::Int(_) | Val::Float(_))) => {
            let n = a.0.borrow().len();
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let l = a.0.borrow()[i];
                out.push(bin(l, ctx, s, op)?);
            }
            Val::Array(ctx.new_array(out))
        }
        (op, s @ (Val::Int(_) | Val::Float(_)), Val::Array(b)) => {
            let n = b.0.borrow().len();
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let r = b.0.borrow()[i];
                out.push(bin(s, ctx, r, op)?);
            }
            Val::Array(ctx.new_array(out))
        }
        (BinOp::Identity, left, right) => Val::Bool(left == right),
        _ => Err(RtErr::invalid_bin(this, op, other))?,
    })
}

pub fn unary<'gc>(this: Val<'gc>, ctx: Ctx<'gc>, op: UnaryOp) -> RtResult<Val<'gc>> {
    fn float_unary<'gc>(op: UnaryOp, value: f64) -> RtResult<Val<'gc>> {
        Ok(match op {
            UnaryOp::Negative => Val::Float(-value),
            UnaryOp::Positive => Val::Float(value),
            _ => Err(RtErr::InvalidUnaryOperand)?,
        })
    }

    fn int_unary<'gc>(op: UnaryOp, value: i64) -> RtResult<Val<'gc>> {
        Ok(match op {
            UnaryOp::BitwiseNot => Val::Int(!value),
            UnaryOp::Negative => Val::Int(value.checked_neg().ok_or(RtErr::IntegerOverflow)?),
            UnaryOp::Positive => Val::Int(value.checked_abs().ok_or(RtErr::IntegerOverflow)?),
            _ => Err(RtErr::InvalidUnaryOperand)?,
        })
    }

    Ok(match (op, this) {
        (UnaryOp::Not, Val::Bool(value)) => Val::Bool(!value),
        (op, Val::Float(value)) => float_unary(op, value)?,
        (op, Val::Int(value)) => int_unary(op, value)?,
        (op, Val::Array(a)) => {
            let n = a.0.borrow().len();
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let v = a.0.borrow()[i];
                out.push(unary(v, ctx, op)?);
            }
            Val::Array(ctx.new_array(out))
        }
        _ => Err(RtErr::InvalidUnaryOperand)?,
    })
}
