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
    /// The unit type, written `()`: truly nothing, the result of an expression that was never
    /// going to hand back a value. An empty block, a `for` loop, and a function with no return all
    /// yield `()`. This is not [Ty::Null], where there _could_ be a value but right now there
    /// isn't.
    ///
    /// ```mimas
    /// let a = {}; // `{}` yields nothing, so `a` is `()`.
    /// ```
    ///
    /// See more in the [book](https://mim.as/reference/special-types.html#unit--).
    #[default]
    Unit,
    /// The never type, written `!`: the type of an expression that diverges, meaning no code can
    /// run after it. It arises from `return`, `panic`, `todo`, and a `loop {}` that never breaks.
    /// Because a diverging branch can never actually supply a value, `!` coerces into any type.
    ///
    /// ```mimas
    /// let a: int = if foo() {
    ///     1
    /// } else {
    ///     return; // `return` is `!`, so it fits where an `int` is expected
    /// };
    /// ```
    ///
    /// See more in the [book](https://mim.as/reference/special-types.html#never--).
    Never,
    /// The type of `null`: there _could_ be a value, but right now there isn't. It only lives
    /// behind a [Ty::Option] -- where it meets another type, the two settle on that type's option,
    /// so `[0, null]` is `[int?]`.
    ///
    /// See more in the [book](https://mim.as/reference/options.html#creating-options).
    Null,
    /// `true` or `false`. Comparisons and logical operators produce `bool`s, and conditions in
    /// `if`, `while`, and friends must be `bool`.
    ///
    /// See more in the [book](https://mim.as/reference/basic-types.html#booleans).
    Bool,
    /// A 64-bit signed integer, an `i64` at runtime. Arithmetic is checked: overflowing the range
    /// is a runtime error rather than silently wrapping.
    ///
    /// See more in the [book](https://mim.as/reference/basic-types.html#integers).
    Int,
    /// A 64-bit IEEE-754 floating-point number, an `f64` at runtime. An `int` combined with a
    /// `float` is promoted, so the result is a `float`, and `/` yields a `float` even between two
    /// `int`s.
    ///
    /// See more in the [book](https://mim.as/reference/basic-types.html#floats).
    Float,
    /// Text is always `str`. There's no split between Rust's `&str` and `String`, and no separate
    /// character type -- a single character is a one-character `str`. All strings are interned in
    /// the garbage collector.
    ///
    /// See more in the [book](https://mim.as/reference/basic-types.html#strings).
    Str,
    /// A type variable: a type the solver hasn't narrowed yet. None should remain by the end of a
    /// successful compilation.
    Vid(Vid),
    /// A slot in a native signature that stands for one type, whichever it turns out to be at each
    /// call. mimas has no generics, but native signatures still need to express things like "the
    /// value pushed must match the array's element type." Scripts can't write these; the host
    /// does, through `vm::anon`, whose `T`, `U`, `V`, and `W` are slots 0 through 3.
    ///
    /// ```rust,ignore
    /// #[mimas]
    /// fn push(arr: &mut Vec<anon::T<'gc>>, value: anon::T<'gc>) {
    ///     arr.push(value);
    /// }
    /// ```
    ///
    /// Notice that both the inner type of the `Vec` and the type of `value` are `anon::T`. This is
    /// what makes it different from an any-type -- the `T` enforces that they are the _same_ type,
    /// regardless of what they may be. Slots are local to their call: the solver swaps each one for
    /// a fresh [Ty::Vid], so an `anon::T` in one function has no relation to one in another.
    ///
    /// See more in the [book](https://mim.as/extension/working-with-types.html#anonymous-types).
    Anon(u32),
    /// An ordered, growable sequence of values that all share one type, like a `Vec` in Rust.
    /// Written `[T]`.
    ///
    /// See more in the [book](https://mim.as/reference/collections/arrays.html).
    Array(Box<Ty>),
    /// A hash map from string keys to values of one type, like a `HashMap<String, T>` in Rust.
    /// Written `~{T}`. Any key might be absent, so indexing one yields `T?`.
    ///
    /// See more in the [book](https://mim.as/reference/collections/dictionaries.html).
    Dict(Box<Ty>),
    /// A fixed-length, heterogeneous sequence. Written as a parenthesized list, like `(int, str)`.
    ///
    /// See more in the [book](https://mim.as/reference/collections/tuples.html).
    Tuple(Vec<Ty>),
    /// Anything callable: a top-level `fn`, a method or associated function from an `impl` block,
    /// a closure, a native, or a tuple struct's constructor. Written `(A, B) -> R`.
    ///
    /// See more in the [book](https://mim.as/reference/functions.html).
    Fn(FnHeader),
    /// A user-defined type: a `struct` (a product type) or an `enum` (a sum type). Modules are
    /// adts internally too, flagged `IS_MODULE`.
    ///
    /// See more in the [book](https://mim.as/reference/types.html).
    Adt(AdtId),
    /// A value that is either `T` or `null`, written `T?`. There's no option of an option: a `T??`
    /// flattens to `T?`.
    ///
    /// See more in the [book](https://mim.as/reference/options.html).
    Option(Box<Ty>),
    /// A recoverable failure, written `T!`: either a success carrying `T`, or an error produced by
    /// `raise`. The error is always a `str` for now.
    ///
    /// See more in the [book](https://mim.as/reference/error-handling.html#results).
    Result(Box<Ty>),
    /// `Self` inside an `impl` block: the adt being implemented. References to an adt within its
    /// own impl items are rewritten to this by `filter_adt`, and it's otherwise interchangeable
    /// with a [Ty::Adt] of the same id.
    ///
    /// See more in the [book](https://mim.as/reference/types/structs.html#methods-and-associated-items).
    Identity(AdtId),
    /// A pact bound: any value whose type implements every pact listed. Written as a pact's name,
    /// or several joined with `+` (`Named + Aged`). The concrete type isn't known statically, so
    /// method calls through a bound dispatch at runtime.
    ///
    /// See more in the [book](https://mim.as/reference/pacts.html).
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
