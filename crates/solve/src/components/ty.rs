use crate::{
    Result, Solver, Unification, UnificationError,
    components::AdtFlags,
    errors::{FieldNotFound, NotAPact, NotAStruct, TypeHasNoFields},
    traits::Query,
};
use parse::{components::Annotation, lex::TyKw};
pub use shared::{FnHeader, FnParam, PactId, Ty, Vid};

pub trait TyExt: Sized {
    fn from_annotation(annotation: Annotation, solver: &mut Solver) -> Result<Ty>;
    /// Finds a common type for expression results, committing successful inference bindings.
    fn join(self, other: Ty, solver: &mut Solver) -> std::result::Result<Ty, UnificationError>;
    fn coerce_pacts(a: Ty, b: Ty, solver: &mut Solver) -> Option<Ty>;
    fn fulfill_ty(
        &mut self,
        other: &mut Ty,
        solver: &mut Solver,
    ) -> std::result::Result<(), UnificationError>;
    fn normalized(self, solver: &Solver) -> Ty;
}

impl TyExt for Ty {
    fn join(self, other: Ty, solver: &mut Solver) -> std::result::Result<Ty, UnificationError> {
        let mut a = self.normalized(solver);
        let mut b = other.normalized(solver);
        match (a.clone(), b.clone()) {
            (Ty::Never, ty) | (ty, Ty::Never) => return Ok(ty),
            (Ty::Option(a), Ty::Option(b)) => {
                return Ok(Ty::Option(Box::new(a.join(*b, solver)?)).normalized(solver));
            }
            (Ty::Option(inner), ty) | (ty, Ty::Option(inner)) => {
                let inner = if ty == Ty::Null {
                    *inner
                } else {
                    inner.join(ty, solver)?
                };
                return Ok(Ty::Option(Box::new(inner)).normalized(solver));
            }
            (Ty::Null, ty) | (ty, Ty::Null) if ty != Ty::Null => {
                return Ok(Ty::Option(Box::new(ty)));
            }
            (Ty::Result(a), Ty::Result(b)) => return Ok(Ty::Result(Box::new(a.join(*b, solver)?))),
            (Ty::Result(inner), ty) | (ty, Ty::Result(inner)) => {
                return Ok(Ty::Result(Box::new(inner.join(ty, solver)?)));
            }
            _ => {}
        }
        match Unification::equate(&mut a, &mut b, solver) {
            Ok(sub) => {
                sub.commit(solver);
                Ok(a.normalized(solver))
            }
            Err(error) => {
                if let Some(ty) = Ty::coerce_pacts(a.clone(), b.clone(), solver) {
                    return Ok(ty);
                }
                if let Ok(sub) = Unification::unify(&mut a, &mut b, solver) {
                    sub.commit(solver);
                    return Ok(b.normalized(solver));
                }
                if let Ok(sub) = Unification::unify(&mut b, &mut a, solver) {
                    sub.commit(solver);
                    return Ok(a.normalized(solver));
                }
                Err(error)
            }
        }
    }

    fn coerce_pacts(a: Ty, b: Ty, solver: &mut Solver) -> Option<Ty> {
        fn bounds(ty: &Ty, solver: &Solver) -> Option<Vec<PactId>> {
            match ty {
                Ty::Adt(aid) => Some(
                    solver
                        .pact_impls
                        .iter()
                        .filter(|(_, impl_adt)| impl_adt == aid)
                        .map(|(pid, _)| *pid)
                        .collect(),
                ),
                _ => ty.as_pacts(),
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
            Annotation::Poison(poison) => poison.escaped(),
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
            Annotation::Ty(ident) => {
                let ty = ident.query(solver)?;
                let dec = solver.ribs.resolve(&ident);
                solver.note(&ident, ty.clone(), dec);
                Ok(ty)
            }
            Annotation::Path(segments) => {
                let mut iter = segments.into_iter();
                let head = iter
                    .next()
                    .expect("path annotation has at least two segments");
                let mut ty = head.query(solver)?;
                let dec = solver.ribs.resolve(&head);
                solver.note(&head, ty.clone(), dec);
                for segment in iter {
                    let adt = match ty.clone().normalized(solver) {
                        Ty::Adt(adt) => adt,
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
                    solver.check_vis(field.dec, segment.location)?;
                    solver.note(&segment, field.ty.clone(), Some(field.dec));
                    ty = field.ty;
                }
                Ok(ty.normalized(solver))
            }
            Annotation::Bounds(idents) => {
                let mut pacts = Vec::with_capacity(idents.len());
                for ident in idents {
                    let ty = ident.query(solver)?;
                    let dec = solver.ribs.resolve(&ident);
                    solver.note(&ident, ty.clone(), dec);
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
        Unification::unify(self, other, solver).map(|v| v.commit(solver))
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
            Ty::Skolem(pid) => Ty::Skolem(pid),
            Ty::Anon(n) => Ty::Anon(n),
            Ty::Option(inner) => match inner.normalized(solver) {
                ty @ Ty::Option(_) => ty,
                ty => Ty::Option(Box::new(ty)),
            },
            Ty::Result(inner) => Ty::Result(Box::new(inner.normalized(solver))),
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
