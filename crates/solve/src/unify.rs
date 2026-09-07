use crate::{
    Error, Solver,
    components::{Ty, Vid},
    errors::TypeMismatch,
};
use shared::Location;

pub(crate) struct Unification;
impl Unification {
    pub(crate) fn unify(
        ty: &mut Ty,
        rule: &mut Ty,
        solver: &Solver,
    ) -> Result<Substitution, UnificationError> {
        #[cfg(feature = "logging")]
        println!("{}", crate::utils::Printer::ty_unification(ty, rule));

        // todo, borrow checker driving me insane
        let potential_err = UnificationError {
            found: ty.to_string(),
            expected: rule.to_string(),
        };

        match (ty, rule) {
            // Directly equal types
            (ty, rule) if ty == rule => Ok(Substitution::None),

            // Unbound Vid unification
            (Ty::Vid(vid), other) | (other, Ty::Vid(vid)) => {
                Ok(Substitution::Single(*vid, other.clone()))
            }

            // Never is never checked
            (Ty::Never, _) | (_, Ty::Never) => Ok(Substitution::None),

            // Adts (or Identities) must point to the same definition
            (Ty::Adt(lhs_adt), Ty::Adt(rhs_adt))
            | (Ty::Identity(lhs_adt), Ty::Adt(rhs_adt))
            | (Ty::Adt(lhs_adt), Ty::Identity(rhs_adt))
                if lhs_adt == rhs_adt =>
            {
                Ok(Substitution::None)
            }

            (Ty::Adt(aid), Ty::Pacts(pids))
            | (Ty::Pacts(pids), Ty::Adt(aid))
            | (Ty::Identity(aid), Ty::Pacts(pids))
            | (Ty::Pacts(pids), Ty::Identity(aid)) => {
                if pids
                    .iter()
                    .all(|pid| solver.pact_impls.contains(&(*pid, *aid)))
                {
                    Ok(Substitution::None)
                } else {
                    Err(potential_err)
                }
            }

            // fn types unify structurally -- param names and the synthesized is_ctor flag
            // don't affect compatibility, only the per-position param types + return type.
            (Ty::Fn(lhs), Ty::Fn(rhs)) if lhs.parameters.len() == rhs.parameters.len() => {
                let mut sub = Substitution::None;
                for (l, r) in lhs.parameters.iter_mut().zip(rhs.parameters.iter_mut()) {
                    sub = sub.combo(
                        Self::unify(&mut l.ty, &mut r.ty, solver)
                            .map_err(|_| potential_err.clone())?,
                    );
                }
                sub = sub.combo(
                    Self::unify(lhs.return_ty.as_mut(), rhs.return_ty.as_mut(), solver)
                        .map_err(|_| potential_err)?,
                );
                Ok(sub)
            }

            // Collections
            (Ty::Array(ty), Ty::Array(exp_ty)) | (Ty::Dict(ty), Ty::Dict(exp_ty)) => {
                Self::unify(ty, exp_ty, solver).map_err(|_| potential_err)
            }
            (Ty::Tuple(lhs), Ty::Tuple(rhs)) if lhs.len() == rhs.len() => lhs
                .iter_mut()
                .zip(rhs.iter_mut())
                .try_fold(Substitution::None, |sub, (l, r)| {
                    Self::unify(l, r, solver).map(|v| sub.combo(v))
                })
                .map_err(|_| potential_err),
            (Ty::Option(ty), Ty::Option(exp)) => {
                Self::unify(ty, exp, solver).or(Err(potential_err))
            }
            (Ty::Result(ty), Ty::Result(exp)) => {
                Self::unify(ty, exp, solver).or(Err(potential_err))
            }

            // ? fulfills T?
            (Ty::Null, Ty::Option(_)) | (Ty::Option(_), Ty::Null) => Ok(Substitution::None),

            // T fulfills T?
            (ty, Ty::Option(rule)) => Self::unify(ty, rule.as_mut(), solver),

            // T fulfills T! (auto-wrap as ok)
            (ty, Ty::Result(rule)) => Self::unify(ty, rule.as_mut(), solver),

            // Anything else is a mismatch
            _ => Err(potential_err),
        }
    }
}

#[derive(Debug)]
#[must_use]
pub(crate) enum Substitution {
    Single(Vid, Ty),
    Multi(Vec<(Vid, Ty)>),
    None,
}
impl Substitution {
    pub(crate) fn combo(self, other: Self) -> Self {
        let mut vec = match other {
            Substitution::Single(vid, ty) => vec![(vid, ty)],
            Substitution::Multi(vec) => vec,
            Substitution::None => vec![],
        };
        match self {
            Substitution::Single(vid, ty) => vec.push((vid, ty)),
            Substitution::Multi(mut other) => vec.append(&mut other),
            Substitution::None => {}
        }
        Substitution::Multi(vec)
    }

    pub(crate) fn commit(self, solver: &mut Solver) -> Result<(), UnificationError> {
        match self {
            Substitution::Single(vid, ty) => solver.register_sub(vid, ty),
            Substitution::Multi(multi) => multi
                .into_iter()
                .try_for_each(|(vid, ty)| solver.register_sub(vid, ty)),
            Substitution::None => Ok(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnificationError {
    found: String,
    expected: String,
}
impl UnificationError {
    pub(crate) fn recursive(vid: Vid, ty: &Ty) -> Self {
        Self {
            found: ty.to_string(),
            expected: format!(
                "a non-recursive type (`{}` occurs within it)",
                crate::utils::Printer::vid(&vid)
            ),
        }
    }

    pub(crate) fn into_type_mismatch(self, solver: &Solver, location: Location) -> Error {
        TypeMismatch {
            src: solver.src(location),
            at: location.into(),
            expected: crate::errors::elide(self.expected),
            found: crate::errors::elide(self.found),
        }
        .into()
    }
}
