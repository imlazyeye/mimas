use crate::{
    Result, Solver, Unification, UnificationError,
    components::AdtFlags,
    errors::{FieldNotFound, NotAPact, NotAStruct, TypeHasNoFields},
    traits::Query,
};
use parse::{components::Annotation, lex::TyKw};
pub use shared::{AdtId, FnHeader, FnParam, PactId, Ty, Vid};

pub trait TyExt: Sized {
    fn occurs(&self, other: Vid, solver: &Solver) -> bool;
    fn from_annotation(annotation: Annotation, solver: &mut Solver) -> Result<Ty>;
    fn coerce_option(a: Ty, b: Ty, solver: &mut Solver) -> Option<Ty>;
    fn coerce_pacts(a: Ty, b: Ty, solver: &mut Solver) -> Option<Ty>;
    fn fulfill_ty(
        &mut self,
        other: &mut Ty,
        solver: &mut Solver,
    ) -> std::result::Result<(), UnificationError>;
    fn normalized(self, solver: &Solver) -> Ty;
    fn filter_adt(&self, adt: AdtId) -> Ty;
}

impl TyExt for Ty {
    fn occurs(&self, other: Vid, solver: &Solver) -> bool {
        match self {
            Ty::Vid(vid) if *vid == other => true,
            Ty::Vid(vid) => solver.sub(*vid).is_some_and(|v| v.occurs(other, solver)),
            Ty::Array(ty) | Ty::Dict(ty) => ty.occurs(other, solver),
            Ty::Fn(fn_data) => {
                fn_data
                    .parameters
                    .iter()
                    .any(|p| p.ty.occurs(other, solver))
                    || fn_data.return_ty.occurs(other, solver)
            }
            Ty::Tuple(members) => members.iter().any(|v| v.occurs(other, solver)),
            // adt bodies are nominal: descending into their field types would loop forever on
            // recursive adts (e.g. `enum Tree { Branch(Vec<Tree>) }`), and a vid inside a field
            // type can't form an infinite type just by being bound to a `Ty::Adt(_)` -- the adt
            // itself doesn't unfold.
            Ty::Adt(_) => false,
            Ty::Option(inner) | Ty::Result(inner) => inner.occurs(other, solver),
            Ty::Identity(_)
            | Ty::Anon(_)
            | Ty::Pacts(_) // annotations always required, never holds vids
            | Ty::Unit
            | Ty::Never
            | Ty::Null
            | Ty::Bool
            | Ty::Int
            | Ty::Float
            | Ty::Str => false,
        }
    }

    fn coerce_option(a: Ty, b: Ty, solver: &mut Solver) -> Option<Ty> {
        match (a, b) {
            (Ty::Null, ty) | (ty, Ty::Null) => {
                if ty.clone().normalized(solver) != Ty::Null {
                    Some(Ty::Option(Box::new(ty)))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Widen two otherwise-incompatible types to the pacts they share, if any. Without
    /// generics, this is the only way to build a collection of "things that implement X":
    /// `[Square, Circle]` settles on `[Draw]` rather than forcing every element into the
    /// first one's concrete type. Only consulted after unification has already failed, so
    /// it can turn an error into a success but never change a program that already checked.
    fn coerce_pacts(a: Ty, b: Ty, solver: &mut Solver) -> Option<Ty> {
        fn bounds(ty: &Ty, solver: &Solver) -> Option<Vec<PactId>> {
            match ty {
                Ty::Adt(aid) | Ty::Identity(aid) => Some(
                    solver
                        .pact_impls
                        .iter()
                        .filter(|(_, impl_adt)| impl_adt == aid)
                        .map(|(pid, _)| *pid)
                        .collect(),
                ),
                Ty::Pacts(pids) => Some(pids.clone()),
                _ => None,
            }
        }

        let a = a.normalized(solver);
        let b = b.normalized(solver);
        let (lhs, rhs) = (bounds(&a, solver)?, bounds(&b, solver)?);
        let shared: Vec<_> = lhs.into_iter().filter(|pid| rhs.contains(pid)).collect();
        // `Ty::pacts` sorts and dedups -- `Ty::Pacts` compares as a plain Vec, so that
        // normalization is what lets two independently derived bounds come out equal
        (!shared.is_empty()).then(|| Ty::pacts(shared))
    }

    fn from_annotation(annotation: Annotation, solver: &mut Solver) -> Result<Ty> {
        match annotation {
            Annotation::Unit => Ok(Ty::Unit),
            Annotation::Kw(kw) => Ok(ty_from_kw(kw)),
            Annotation::Option(ty) => Ok(Ty::Option(Box::new(Ty::from_annotation(*ty, solver)?))),
            Annotation::Result(ty) => Ok(Ty::Result(Box::new(Ty::from_annotation(*ty, solver)?))),
            Annotation::Array(ty) => Ok(Ty::Array(Box::new(Ty::from_annotation(*ty, solver)?))),
            Annotation::Dictionary(ty) => Ok(Ty::Dict(Box::new(Ty::from_annotation(*ty, solver)?))),
            Annotation::Function(params, ret) => {
                let parameters = params
                    .into_iter()
                    .map(|p| Ty::from_annotation(p, solver).map(|t| FnParam::new(None, t, false)))
                    .collect::<Result<_>>()?;
                let return_ty = Ty::from_annotation(*ret, solver)?;
                Ok(Ty::Fn(FnHeader::new(parameters, return_ty, false)))
            }
            Annotation::Tuple(members) => Ok(Ty::Tuple(
                members
                    .into_iter()
                    .map(|v| Ty::from_annotation(v, solver))
                    .collect::<Result<_>>()?,
            )),
            Annotation::Ty(ident) => ident.query(solver),
            Annotation::Path(segments) => {
                let mut iter = segments.into_iter();
                let head = iter
                    .next()
                    .expect("path annotation has at least two segments");
                let mut ty = head.query(solver)?;
                for segment in iter {
                    let adt = match ty.clone().normalized(solver) {
                        Ty::Adt(adt) | Ty::Identity(adt) => adt,
                        other => Err(TypeHasNoFields {
                            src: solver.src(segment.location),
                            at: segment.location.into(),
                            ty: other.to_string(),
                        })?,
                    };
                    if !solver.adts[adt].flags.contains(AdtFlags::IS_MODULE) {
                        Err(NotAStruct {
                            src: solver.src(segment.location),
                            at: segment.location.into(),
                            ty: solver.adts[adt].name.clone(),
                        })?
                    }
                    let field = solver.adts[adt]
                        .as_struct()
                        .fields
                        .get(&segment.lexeme)
                        .cloned()
                        .ok_or_else(|| FieldNotFound {
                            src: solver.src(segment.location),
                            at: segment.location.into(),
                            field_name: segment.lexeme.clone(),
                        })?;
                    ty = field.ty;
                }
                Ok(ty.normalized(solver))
            }
            Annotation::Bounds(idents) => {
                let mut pacts = Vec::with_capacity(idents.len());
                for ident in idents {
                    let ty = ident.query(solver)?;
                    let Some(pid) = ty.as_single_pact() else {
                        return Err(NotAPact {
                            src: solver.src(ident.location),
                            at: ident.location.into(),
                            ty: ty.to_string(),
                        }
                        .into());
                    };
                    pacts.push(pid);
                }
                Ok(Ty::pacts(pacts))
            }
        }
    }

    fn fulfill_ty(
        &mut self,
        other: &mut Ty,
        solver: &mut Solver,
    ) -> std::result::Result<(), UnificationError> {
        Unification::unify(self, other, solver).and_then(|v| v.commit(solver))
    }

    fn normalized(self, solver: &Solver) -> Ty {
        match self {
            Ty::Unit | Ty::Never | Ty::Null | Ty::Bool | Ty::Int | Ty::Float | Ty::Str => self,
            Ty::Vid(vid) => solver
                .sub(vid)
                .map(|ty| ty.clone().normalized(solver))
                .unwrap_or(Ty::Vid(vid)),
            Ty::Array(ty) => Ty::Array(Box::new(ty.normalized(solver))),
            Ty::Dict(ty) => Ty::Dict(Box::new(ty.normalized(solver))),
            Ty::Tuple(members) => {
                Ty::Tuple(members.into_iter().map(|v| v.normalized(solver)).collect())
            }
            Ty::Fn(f) => {
                let was_ctor = f.is_ctor;
                let mut h = FnHeader::new(
                    f.parameters
                        .into_iter()
                        .map(|p| FnParam::new(p.name, p.ty.normalized(solver), p.has_default))
                        .collect(),
                    f.return_ty.normalized(solver),
                    f.is_method,
                );
                h.is_ctor = was_ctor;
                Ty::Fn(h)
            }
            Ty::Adt(adt) => Ty::Adt(adt),
            Ty::Pacts(pacts) => Ty::Pacts(pacts),
            Ty::Anon(n) => Ty::Anon(n),
            Ty::Identity(adt) => Ty::Identity(adt),
            Ty::Option(inner) => {
                if let Ty::Option(nested_inner) = *inner {
                    Ty::Option(Box::new(nested_inner.normalized(solver)))
                } else {
                    Ty::Option(Box::new(inner.normalized(solver)))
                }
            }
            Ty::Result(inner) => Ty::Result(Box::new(inner.normalized(solver))),
        }
    }

    fn filter_adt(&self, adt: AdtId) -> Ty {
        match self {
            Ty::Array(ty) => Ty::Array(Box::new(ty.filter_adt(adt))),
            Ty::Dict(ty) => Ty::Dict(Box::new(ty.filter_adt(adt))),
            Ty::Tuple(members) => Ty::Tuple(members.iter().map(|m| m.filter_adt(adt)).collect()),
            Ty::Adt(this_adt) if *this_adt == adt => Ty::Identity(adt),
            Ty::Fn(f) => {
                let parameters: Vec<FnParam> = f
                    .parameters
                    .iter()
                    .map(|p| FnParam::new(p.name.clone(), p.ty.filter_adt(adt), p.has_default))
                    .collect();
                let mut h = FnHeader::new(parameters, f.return_ty.filter_adt(adt), f.is_method);
                h.is_ctor = f.is_ctor;
                Ty::Fn(h)
            }
            Ty::Option(inner) => Ty::Option(Box::new(inner.filter_adt(adt))),
            Ty::Result(inner) => Ty::Result(Box::new(inner.filter_adt(adt))),
            Ty::Adt(_)
            | Ty::Pacts(_)
            | Ty::Identity(_)
            | Ty::Anon(_)
            | Ty::Vid(_)
            | Ty::Unit
            | Ty::Never
            | Ty::Null
            | Ty::Bool
            | Ty::Int
            | Ty::Float
            | Ty::Str => self.clone(),
        }
    }
}

pub(crate) fn ty_from_kw(kw: TyKw) -> Ty {
    match kw {
        TyKw::Int => Ty::Int,
        TyKw::Float => Ty::Float,
        TyKw::Str => Ty::Str,
        TyKw::Bool => Ty::Bool,
    }
}
