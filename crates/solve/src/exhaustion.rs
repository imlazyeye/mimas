//! Maranget-style usefulness check for `match` exhaustion.
//!
//! Given a list of `MatchCase`s and the scrutinee's type, this answers the question: "does any
//! possible value of the scrutinee fail to match any case?" If yes -> not exhaustive (result type
//! becomes `T?`). If no -> exhaustive (result type stays `T`).
//!
//! The algorithm follows Luc Maranget's 2007 paper "Warnings for pattern matching" (JFP) (well, no,
//! we actually follow rustc's explanation in `usefullness.rs`, which was much more digestible, but
//! _they_ mostly followed that paper!) We build a pattern matrix (one row per non-guarded case),
//! then ask whether a hypothetical wildcard would be "useful" -- i.e. would match some value none
//! of the rows match. If a wildcard is NOT useful, every value is covered, so the match is
//! exhaustive.
//!
//! Guarded rows are dropped from the matrix entirely: a guard might evaluate false, leaving
//! values uncovered, so we can't count those rows as contributing to exhaustion.
//!
//! Constructors (`Ctor`) abstract over the "shape" of a value at one position. A finite type
//! (bool, enum) has a complete signature when every constructor is seen, tuples and structs
//! are always complete (one constructor), primitives like int/float/str are never complete
//! through enumeration (only wildcards exhaust them).
//!
//! tldr we did it joe!

use crate::{
    Solver,
    components::{AdtFlags, AdtId, Ty, TyExt, Variant},
    errors::VariantDefinedHere,
};
use parse::{
    Literal as LitVal, MatchCase,
    components::{Pat, PatKind},
};
use shared::Located;
use std::{
    collections::HashSet,
    ops::{Deref, DerefMut},
};

/// Top-level entry. Enumerates uncovered example values up to a safety cap so the diagnostic
/// can list "and N more" -- see `MissingReport`. `None` means the match is exhaustive.
pub(crate) fn missing_witnesses(
    cases: &[MatchCase],
    scrut_ty: &Ty,
    solver: &Solver,
) -> Option<MissingReport> {
    let mut buf: Vec<Vec<Witness>> = Vec::new();
    Matrix::for_cases(cases).collect_witnesses(std::slice::from_ref(scrut_ty), solver, &mut buf);
    if buf.is_empty() {
        return None;
    }
    let total = buf.len();
    let capped = total >= WITNESS_CAP;
    let variant_defs = buf
        .iter()
        .flatten()
        .find_map(|w| first_variant_def(w, solver))
        .into_iter()
        .collect();
    let shown: Vec<String> = buf
        .into_iter()
        .take(2)
        .map(|w| render_witness(w.into_iter().next().unwrap_or(Witness::Wildcard), solver))
        .collect();
    let others = total.saturating_sub(shown.len());
    Some(MissingReport {
        shown,
        others,
        capped,
        variant_defs,
    })
}

/// First source-defined enum variant named anywhere in a witness tree, as a label pointing at its
/// definition in the enum. Native variants (no stored span) and non-variant ctors are skipped.
fn first_variant_def(w: &Witness, solver: &Solver) -> Option<VariantDefinedHere> {
    let Witness::Ctor(ctor, subs) = w else {
        return None;
    };
    if let Ctor::Variant(adt, name) = ctor
        && let Some(loc) = solver.adts[adt]
            .variants
            .get(name)
            .and_then(Variant::location)
    {
        return Some(VariantDefinedHere {
            src: solver.src(loc),
            at: loc.into(),
            name: format!("{}::{}", solver.adts[adt].name, name),
        });
    }
    subs.iter().find_map(|s| first_variant_def(s, solver))
}

const WITNESS_CAP: usize = 50;

pub(crate) struct MissingReport {
    pub shown: Vec<String>,
    pub others: usize,
    pub capped: bool,
    pub variant_defs: Vec<VariantDefinedHere>,
}

impl MissingReport {
    /// Formatted summary suitable for embedding in a diagnostic -- e.g.
    /// `` missing pattern `false` `` or `` missing patterns `A`, `B`, and 7 others ``.
    pub fn summary(&self) -> String {
        let mut s = String::from("missing ");
        s.push_str(if self.shown.len() == 1 && self.others == 0 {
            "pattern "
        } else {
            "patterns "
        });
        let quoted: Vec<String> = self.shown.iter().map(|w| format!("`{w}`")).collect();
        if self.others == 0 {
            s.push_str(&quoted.join(" and "));
        } else {
            s.push_str(&quoted.join(", "));
            s.push_str(", and ");
            if self.capped {
                s.push_str("many others");
            } else {
                s.push_str(&format!(
                    "{} other{}",
                    self.others,
                    if self.others == 1 { "" } else { "s" }
                ));
            }
        }
        s
    }
}

#[derive(Debug, Clone)]
enum Witness {
    Ctor(Ctor, Vec<Witness>),
    Wildcard,
}

fn render_witness(w: Witness, solver: &Solver) -> String {
    match w {
        Witness::Wildcard => "_".into(),
        Witness::Ctor(ctor, subs) => match ctor {
            Ctor::Bool(true) => "true".into(),
            Ctor::Bool(false) => "false".into(),
            Ctor::Null => "null".into(),
            Ctor::Lit => "_".into(),
            Ctor::Some => format!(
                "{}?",
                render_witness(subs.into_iter().next().unwrap_or(Witness::Wildcard), solver)
            ),
            Ctor::Tuple(_) => format!(
                "({})",
                subs.into_iter()
                    .map(|s| render_witness(s, solver))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Ctor::Variant(adt, name) => {
                let adt_data = &solver.adts[adt];
                let qualified = format!("{}::{}", adt_data.name, name);
                let variant = &adt_data.variants[&name];
                render_adt_witness(&qualified, variant, subs, solver)
            }
            Ctor::Single(adt) => {
                let adt_data = &solver.adts[adt];
                let variant = adt_data.variants.values().next().unwrap();
                render_adt_witness(&adt_data.name, variant, subs, solver)
            }
        },
    }
}

fn render_adt_witness(
    name: &str,
    variant: &Variant,
    subs: Vec<Witness>,
    solver: &Solver,
) -> String {
    match variant {
        Variant::Tuple(tv) if tv.members.is_empty() => name.into(),
        Variant::Tuple(_) => format!(
            "{}({})",
            name,
            subs.into_iter()
                .map(|s| render_witness(s, solver))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Variant::Struct(sv) => {
            let fields: Vec<_> = sv
                .fields
                .keys()
                .zip(subs)
                .map(|(k, w)| format!("{}: {}", k, render_witness(w, solver)))
                .collect();
            if fields.is_empty() {
                name.into()
            } else {
                format!("{} {{ {} }}", name, fields.join(", "))
            }
        }
    }
}

/// Constructors abstract over the head shape at one position. The "signature" of a type is the
/// set of constructors that, taken together, cover every value of the type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Ctor {
    /// `true` or `false`. The pair completes `bool`'s signature.
    Bool(bool),
    /// One named variant of an enum adt.
    Variant(AdtId, String),
    /// The sole constructor of a non-enum adt (struct / tuple-struct).
    Single(AdtId),
    /// Fixed-arity tuple. There's only one constructor per arity.
    Tuple(usize),
    /// `null` -- the absent half of an `Option<T>`.
    Null,
    /// The present half of an `Option<T>`, carrying its payload.
    Some,
    /// Any literal whose value-space we don't enumerate (int / float / str). A `Lit` never
    /// completes a signature -- only wildcards can exhaust these types.
    Lit,
}

impl Ctor {
    /// The constructors that together cover every value of `ty`, or `None` if `ty`'s signature
    /// can't be enumerated (int / float / str / unresolved type vars).
    fn signature_for(ty: &Ty, solver: &Solver) -> Option<Vec<Self>> {
        match ty {
            Ty::Bool => Some(vec![Ctor::Bool(true), Ctor::Bool(false)]),
            Ty::Option(_) => Some(vec![Ctor::Null, Ctor::Some]),
            Ty::Tuple(members) => Some(vec![Ctor::Tuple(members.len())]),
            Ty::Adt(adt) | Ty::Identity(adt) => {
                let flags = solver.adts[adt].flags;
                if flags.contains(AdtFlags::IS_ENUM) {
                    Some(
                        solver.adts[adt]
                            .variants
                            .keys()
                            .map(|n| Ctor::Variant(*adt, n.clone()))
                            .collect(),
                    )
                } else if flags.contains(AdtFlags::IS_MODULE) {
                    None
                } else {
                    Some(vec![Ctor::Single(*adt)])
                }
            }
            _ => None,
        }
    }

    /// Looks up the head constructor of a pattern, plus its subpatterns (in canonical order for
    /// this constructor). Returns `None` for binding/wildcard patterns. Or-patterns return None
    /// at this level and are flattened by callers.
    fn from_pat(pat: &Pat, ty: &Ty, solver: &Solver) -> Option<(Self, Vec<Pat>)> {
        match pat.kind() {
            PatKind::Ident(_) | PatKind::Or(_) => None,
            PatKind::NullBind(pat) => Some((Ctor::Some, vec![pat.as_ref().clone()])),
            PatKind::Literal(lit) => match lit {
                LitVal::True => Some((Ctor::Bool(true), vec![])),
                LitVal::False => Some((Ctor::Bool(false), vec![])),
                LitVal::Null => Some((Ctor::Null, vec![])),
                _ => Some((Ctor::Lit, vec![])),
            },
            PatKind::Tuple(ps) => Some((Ctor::Tuple(ps.len()), ps.clone())),
            PatKind::TupleVariant(_, ps) => {
                Some((Ctor::from_path_pat(pat, ty, solver)?, ps.clone()))
            }
            PatKind::Variant(_) => Some((Ctor::from_path_pat(pat, ty, solver)?, vec![])),
            PatKind::Struct(_, fields) => {
                let ctor = Ctor::from_path_pat(pat, ty, solver)?;
                let (adt, variant_key) = ctor.variant_key(solver)?;
                // Reorder field patterns to match the struct's declaration order; missing fields
                // are filled with synthetic wildcards.
                let ordered: Vec<Pat> = match solver.adts[adt].variants.get(&variant_key) {
                    Some(Variant::Struct(sv)) => sv
                        .fields
                        .keys()
                        .map(|k| {
                            fields
                                .get(k)
                                .cloned()
                                .unwrap_or_else(|| wildcard_pat(pat.location()))
                        })
                        .collect(),
                    _ => vec![],
                };
                Some((ctor, ordered))
            }
        }
    }

    /// Build a `Variant`/`Single` constructor for a path-led pattern (`Foo::Bar`, `Foo { ... }`,
    /// `Foo(..)`), resolving the path against the scrutinee's type.
    fn from_path_pat(pat: &Pat, ty: &Ty, solver: &Solver) -> Option<Self> {
        let path = match pat.kind() {
            PatKind::Variant(p) | PatKind::TupleVariant(p, _) | PatKind::Struct(p, _) => p.as_ref(),
            _ => return None,
        };
        let adt = match ty.clone().normalized(solver) {
            Ty::Adt(adt) | Ty::Identity(adt) => adt,
            _ => return None,
        };
        match path.kind() {
            parse::ExprKind::Access(parse::Access::DoubleColon { right, .. })
                if solver.adts[adt].flags.contains(AdtFlags::IS_ENUM)
                    && solver.adts[adt].variants.contains_key(&right.lexeme) =>
            {
                Some(Ctor::Variant(adt, right.lexeme.clone()))
            }
            parse::ExprKind::Ident(_) => Some(Ctor::Single(adt)),
            _ => None,
        }
    }

    /// The adt and variant-map key for adt-backed constructors. `Variant` carries the key
    /// directly; `Single` looks up the sole variant.
    fn variant_key(&self, solver: &Solver) -> Option<(AdtId, String)> {
        match self {
            Ctor::Variant(adt, name) => Some((*adt, name.clone())),
            Ctor::Single(adt) => Some((
                *adt,
                solver.adts[adt].variants.keys().next().cloned().unwrap(),
            )),
            _ => None,
        }
    }

    /// Subtypes of this constructor's fields, in dec/stable order.
    fn subtypes(&self, parent_ty: &Ty, solver: &Solver) -> Vec<Ty> {
        match self {
            Ctor::Bool(_) | Ctor::Null | Ctor::Lit => vec![],
            Ctor::Some => match parent_ty.clone().normalized(solver) {
                Ty::Option(inner) | Ty::Result(inner) => vec![*inner],
                _ => vec![],
            },
            Ctor::Tuple(_) => match parent_ty.clone().normalized(solver) {
                Ty::Tuple(tys) => tys,
                _ => vec![],
            },
            Ctor::Single(_) | Ctor::Variant(_, _) => {
                let Some((adt, key)) = self.variant_key(solver) else {
                    return vec![];
                };
                match solver.adts[adt].variants.get(&key) {
                    Some(Variant::Tuple(tv)) => tv.members.clone(),
                    Some(Variant::Struct(sv)) => sv.fields.values().map(|f| f.ty.clone()).collect(),
                    None => vec![],
                }
            }
        }
    }
}

type Row = Vec<Pat>;

struct Matrix(Vec<Row>);

impl Deref for Matrix {
    type Target = Vec<Row>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Matrix {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Matrix {
    /// Initial single-column matrix built from a match's non-guarded cases.
    fn for_cases(cases: &[MatchCase]) -> Self {
        Self(
            cases
                .iter()
                .filter(|c| c.guard().is_none())
                .map(|c| vec![c.pat().clone()])
                .collect(),
        )
    }

    /// One row whose first column was `Or(alts)`, expanded into N rows each with one alt up
    /// front. Used when downstream methods hit an Or in column 0.
    fn from_or_alts(alts: &[Pat], rest: &[Pat]) -> Self {
        Self(
            alts.iter()
                .map(|alt| {
                    let mut r = vec![alt.clone()];
                    r.extend_from_slice(rest);
                    r
                })
                .collect(),
        )
    }

    /// Specialize by constructor `c`: keep rows whose first pat is `c` (or wildcard), then expand
    /// the first column into `c`'s sub-patterns. Or-patterns in column 0 are flattened on the fly.
    fn specialized(&self, c: &Ctor, parent_ty: &Ty, solver: &Solver) -> Matrix {
        let arity = c.subtypes(parent_ty, solver).len();
        let mut out = Vec::new();
        for pats in self.iter() {
            let Some((first, rest)) = pats.split_first() else {
                continue;
            };
            match first.kind() {
                PatKind::Ident(_) => {
                    let mut new: Vec<Pat> =
                        (0..arity).map(|_| wildcard_pat(first.location())).collect();
                    new.extend_from_slice(rest);
                    out.push(new);
                }
                PatKind::Or(alts) => {
                    out.extend(
                        Matrix::from_or_alts(alts, rest)
                            .specialized(c, parent_ty, solver)
                            .0,
                    );
                }
                _ => {
                    if let Some((ct, mut sub)) = Ctor::from_pat(first, parent_ty, solver)
                        && &ct == c
                    {
                        sub.extend_from_slice(rest);
                        out.push(sub);
                    }
                }
            }
        }
        Self(out)
    }

    /// Keep only rows that start with a wildcard, dropping the first column. Used when a column's
    /// signature can't be enumerated -- only wildcards survive into the recursion.
    fn defaulted(&self) -> Matrix {
        let mut out = Vec::new();
        for pats in self.iter() {
            let Some((first, rest)) = pats.split_first() else {
                continue;
            };
            match first.kind() {
                PatKind::Ident(_) => out.push(rest.to_vec()),
                PatKind::Or(alts) => out.extend(Matrix::from_or_alts(alts, rest).defaulted().0),
                _ => {}
            }
        }
        Self(out)
    }

    /// The constructors that actually appear in the first column (with Or-flattening).
    fn ctors_present(&self, ty: &Ty, solver: &Solver) -> HashSet<Ctor> {
        let mut out = HashSet::new();
        for pats in self.iter() {
            let Some((first, rest)) = pats.split_first() else {
                continue;
            };
            match first.kind() {
                PatKind::Or(alts) => {
                    out.extend(Matrix::from_or_alts(alts, rest).ctors_present(ty, solver));
                }
                _ => {
                    if let Some((c, _)) = Ctor::from_pat(first, ty, solver) {
                        out.insert(c);
                    }
                }
            }
        }
        out
    }

    /// Witness-producing variant of Maranget's `U`. Pushes every distinct uncovered example
    /// (column-by-column) into `out` until `WITNESS_CAP` is reached. An empty `out` after a
    /// top-level call means the matrix is exhaustive.
    fn collect_witnesses(&self, col_tys: &[Ty], solver: &Solver, out: &mut Vec<Vec<Witness>>) {
        if out.len() >= WITNESS_CAP {
            return;
        }
        if col_tys.is_empty() {
            if self.iter().all(|p| !p.is_empty()) {
                out.push(vec![]);
            }
            return;
        }
        let parent_ty = &col_tys[0];
        let rest_tys = &col_tys[1..];

        match Ctor::signature_for(parent_ty, solver) {
            Some(sig) => {
                let seen = self.ctors_present(parent_ty, solver);
                let missing: Vec<&Ctor> = sig.iter().filter(|c| !seen.contains(c)).collect();

                if !missing.is_empty() {
                    let mut rest_buf = Vec::new();
                    self.defaulted()
                        .collect_witnesses(rest_tys, solver, &mut rest_buf);
                    for c in &missing {
                        let arity = c.subtypes(parent_ty, solver).len();
                        let head_subs: Vec<Witness> =
                            (0..arity).map(|_| Witness::Wildcard).collect();
                        for rest in &rest_buf {
                            if out.len() >= WITNESS_CAP {
                                return;
                            }
                            let mut w = vec![Witness::Ctor((*c).clone(), head_subs.clone())];
                            w.extend_from_slice(rest);
                            out.push(w);
                        }
                    }
                }

                for c in sig.iter().filter(|c| seen.contains(c)) {
                    if out.len() >= WITNESS_CAP {
                        return;
                    }
                    let sub = self.specialized(c, parent_ty, solver);
                    let mut new_tys = c.subtypes(parent_ty, solver);
                    let arity = new_tys.len();
                    new_tys.extend_from_slice(rest_tys);
                    let mut sub_buf = Vec::new();
                    sub.collect_witnesses(&new_tys, solver, &mut sub_buf);
                    for inner in sub_buf {
                        if out.len() >= WITNESS_CAP {
                            return;
                        }
                        let (head_subs, rest) = inner.split_at(arity);
                        let mut w = vec![Witness::Ctor(c.clone(), head_subs.to_vec())];
                        w.extend_from_slice(rest);
                        out.push(w);
                    }
                }
            }
            None => {
                let mut rest_buf = Vec::new();
                self.defaulted()
                    .collect_witnesses(rest_tys, solver, &mut rest_buf);
                for rest in rest_buf {
                    if out.len() >= WITNESS_CAP {
                        return;
                    }
                    let mut w = vec![Witness::Wildcard];
                    w.extend_from_slice(&rest);
                    out.push(w);
                }
            }
        }
    }
}

fn wildcard_pat(location: shared::Location) -> Pat {
    Pat::new(
        PatKind::Ident(parse::Ident {
            lexeme: "_".into(),
            location,
        }),
        location,
    )
}
