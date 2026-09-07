use crate::AdtId;
use indexmap::IndexMap;
use itertools::Itertools;

crate::id!(pub Vid, pub PactId);

impl Vid {
    pub const UNKNOWN: Vid = Vid(u32::MAX);
}

/// Prefix used by the Display impls for [Ty::Vid] and [Ty::Anon]. Diagnostic emitters scan rendered
/// messages for this token; if they see it, an unresolved/internal type leaked into something
/// user-facing -- a real mimas bug, not the user's fault. See [INTERNAL_TY_LEAK_NOTE].
pub const INTERNAL_TY_MARKER: &str = "?mimas<";

#[derive(Debug, Clone, Default)]
pub enum Ty {
    #[default]
    Unit,
    Never,
    Null,
    Bool,
    Int,
    Float,
    Str,
    Vid(Vid),
    Anon(u32),
    Array(Box<Ty>),
    Dict(Box<Ty>),
    Tuple(Vec<Ty>),
    Fn(FnHeader),
    Adt(AdtId),
    Option(Box<Ty>),
    Result(Box<Ty>),
    Identity(AdtId),
    Pacts(Vec<PactId>),
}

impl PartialEq for Ty {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Vid(l), Self::Vid(r)) => l == r,
            (Self::Anon(l), Self::Anon(r)) => l == r,
            (Self::Array(l), Self::Array(r)) => l == r,
            (Self::Dict(l), Self::Dict(r)) => l == r,
            (Self::Tuple(l), Self::Tuple(r)) => l == r,
            (Self::Fn(l), Self::Fn(r)) => l == r,
            (Self::Option(l), Self::Option(r)) => l == r,
            (Self::Result(l), Self::Result(r)) => l == r,
            (Self::Adt(l), Self::Adt(r)) => l == r,
            (Self::Identity(l), Self::Identity(r)) => l == r,
            (Self::Pacts(l), Self::Pacts(r)) => l == r,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl From<Vid> for Ty {
    fn from(value: Vid) -> Self {
        Ty::Vid(value)
    }
}

impl Ty {
    /// Builds a pact bound, normalized to a sorted, deduped set so equality is order-insensitive.
    pub fn pacts(mut pacts: Vec<PactId>) -> Ty {
        pacts.sort_by_key(|p| p.index());
        pacts.dedup();
        Ty::Pacts(pacts)
    }

    pub fn as_single_pact(&self) -> Option<PactId> {
        match self {
            Ty::Pacts(pacts) if pacts.len() == 1 => Some(pacts[0]),
            _ => None,
        }
    }

    pub fn as_adt(&self) -> Option<&AdtId> {
        match self {
            Ty::Identity(adt) | Ty::Adt(adt) => Some(adt),
            _ => None,
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Ty::Float | Ty::Int)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pact {
    pub name: String,
    pub functions: IndexMap<String, (FnHeader, Option<Ty>)>,
    pub constants: IndexMap<String, Ty>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnParam {
    /// Source-level name when available (fn definitions). `None` for closures, natives,
    /// and types derived purely by inference where there's no declaration to read.
    pub name: Option<String>,
    pub ty: Ty,
    pub has_default: bool,
}

impl FnParam {
    pub fn new(name: Option<String>, ty: Ty, has_default: bool) -> Self {
        Self {
            name,
            ty,
            has_default,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnHeader {
    pub parameters: Vec<FnParam>,
    pub return_ty: Box<Ty>,
    pub is_method: bool,
    /// Synthesized by the solver when a tuple-struct name is used as a value (`let make = Foo;`).
    /// IR consults it to lower calls to `NewInstance` instead of `CallDirect` on a body that
    /// doesn't exist.
    pub is_ctor: bool,
}

impl FnHeader {
    pub fn new(parameters: Vec<FnParam>, return_ty: Ty, is_method: bool) -> Self {
        Self {
            parameters,
            return_ty: Box::new(return_ty),
            is_method,
            is_ctor: false,
        }
    }

    pub fn ctor(self) -> Self {
        Self {
            is_ctor: true,
            ..self
        }
    }
}

#[mutants::skip]
impl std::fmt::Display for FnHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let param_str = self.parameters.iter().map(|p| &p.ty).join(", ");
        f.pad(&format!("({param_str}) -> {}", self.return_ty))
    }
}

// `Display` has no way to reach the solver's tables, so the solver mirrors adt/pact names
// into this thread-local as it allocates them -- the alternative is `<adt>` placeholders in
// every type error
thread_local! {
    static TY_NAMES: std::cell::RefCell<(Vec<String>, Vec<String>)> =
        const { std::cell::RefCell::new((Vec::new(), Vec::new())) };
}

pub fn name_adt(index: usize, name: &str) {
    TY_NAMES.with_borrow_mut(|(adts, _)| {
        if adts.len() <= index {
            adts.resize(index + 1, String::new());
        }
        adts[index] = name.to_string();
    });
}

pub fn name_pact(index: usize, name: &str) {
    TY_NAMES.with_borrow_mut(|(_, pacts)| {
        if pacts.len() <= index {
            pacts.resize(index + 1, String::new());
        }
        pacts[index] = name.to_string();
    });
}

fn adt_display(index: usize) -> Option<String> {
    TY_NAMES.with_borrow(|(adts, _)| {
        let name = adts.get(index).filter(|n| !n.is_empty())?;
        // module adts are spelled `<module:foo>` internally
        Some(
            match name
                .strip_prefix("<module:")
                .and_then(|r| r.strip_suffix('>'))
            {
                Some(module) => format!("module `{module}`"),
                None => name.clone(),
            },
        )
    })
}

fn pact_display(index: usize) -> Option<String> {
    TY_NAMES.with_borrow(|(_, pacts)| pacts.get(index).filter(|n| !n.is_empty()).cloned())
}

#[mutants::skip]
impl std::fmt::Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Ty::Unit => "()".into(),
            Ty::Never => "!".into(),
            Ty::Null => "null".into(),
            Ty::Bool => "bool".into(),
            Ty::Int => "int".into(),
            Ty::Float => "float".into(),
            Ty::Str => "str".into(),
            Ty::Array(ty) => format!("[{ty}]"),
            Ty::Dict(ty) => format!("~{{{ty}}}"),
            Ty::Tuple(members) => format!("({})", members.iter().map(|v| v.to_string()).join(", ")),
            Ty::Fn(h) => h.to_string(),
            // both vids and anons mark "an internal type slot that should have been resolved
            // before reaching a user-visible message". the shared `?mimas<...>` prefix lets the
            // diag emitter recognize either as a leak and attach an explanatory note.
            Ty::Vid(vid) => format!("{INTERNAL_TY_MARKER}T{}>", vid.index()),
            Ty::Anon(n) => format!("{INTERNAL_TY_MARKER}A{n}>"),
            Ty::Identity(_) => "Self".into(),
            Ty::Adt(id) => adt_display(id.index()).unwrap_or_else(|| "<adt>".into()),
            Ty::Option(inner) => format!("{inner}?"),
            Ty::Result(inner) => format!("{inner}!"),
            Ty::Pacts(ids) => ids
                .iter()
                .map(|p| pact_display(p.index()))
                .collect::<Option<Vec<_>>>()
                .map(|names| names.join(" + "))
                .unwrap_or_else(|| "<pacts>".into()),
        };
        f.pad(&string)
    }
}
