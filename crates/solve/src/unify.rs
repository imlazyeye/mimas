use crate::{
    Error, Solver,
    components::{Ty, Vid},
    errors::TypeMismatch,
};
use indexmap::IndexMap;
use shared::Location;

pub(crate) struct Unification<'a> {
    solver: &'a Solver,
    subs: IndexMap<Vid, Ty>,
}
impl Unification<'_> {
    /// Checks whether `ty` fulfills `rule`, returning bindings without changing the solver.
    pub(crate) fn unify(
        ty: &mut Ty,
        rule: &mut Ty,
        solver: &Solver,
    ) -> Result<Substitution, UnificationError> {
        #[cfg(feature = "logging")]
        println!("{}", crate::utils::Printer::ty_unification(ty, rule));
        let mut trial = Unification {
            solver,
            subs: IndexMap::new(),
        };
        trial.relate(ty, rule, false)?;
        Ok(Substitution(trial.subs))
    }

    /// Checks type equality without applying coercions or committing bindings.
    pub(crate) fn equate(
        ty: &mut Ty,
        rule: &mut Ty,
        solver: &Solver,
    ) -> Result<Substitution, UnificationError> {
        #[cfg(feature = "logging")]
        println!("{}", crate::utils::Printer::ty_unification(ty, rule));
        let mut trial = Unification {
            solver,
            subs: IndexMap::new(),
        };
        trial.relate(ty, rule, true)?;
        Ok(Substitution(trial.subs))
    }

    /// Follows type variable bindings in this trial and the solver.
    fn resolve<'a>(&'a self, ty: &'a Ty) -> &'a Ty {
        match ty {
            Ty::Vid(vid) => self
                .subs
                .get(vid)
                .or_else(|| self.solver.sub(*vid))
                .map_or(ty, |ty| self.resolve(ty)),
            _ => ty,
        }
    }

    /// Checks whether `vid` occurs inside `ty` after following substitutions.
    fn occurs(&self, vid: Vid, ty: &Ty) -> bool {
        match self.resolve(ty) {
            Ty::Vid(other) => *other == vid,
            Ty::Array(inner) | Ty::Dict(inner) | Ty::Option(inner) | Ty::Result(inner) => {
                self.occurs(vid, inner)
            }
            Ty::Tuple(members) => members.iter().any(|ty| self.occurs(vid, ty)),
            Ty::Fn(header) => {
                header.parameters.iter().any(|p| self.occurs(vid, &p.ty))
                    || self.occurs(vid, &header.return_ty)
            }
            _ => false,
        }
    }

    /// Checks compatibility, or equality when `exact`, and records bindings in this trial.
    fn relate(&mut self, found: &Ty, expected: &Ty, exact: bool) -> Result<(), UnificationError> {
        let found = self.resolve(found).clone();
        let expected = self.resolve(expected).clone();
        let error = UnificationError {
            found: found.to_string(),
            expected: expected.to_string(),
        };
        match (&found, &expected) {
            (a, b) if a == b => Ok(()),
            (Ty::Vid(vid), ty) | (ty, Ty::Vid(vid)) => {
                if self.occurs(*vid, ty) {
                    return Err(UnificationError::recursive(*vid, ty));
                }
                self.subs.insert(*vid, ty.clone());
                Ok(())
            }
            (Ty::Never, _) if !exact => Ok(()),
            (Ty::Adt(layout), Ty::Adt(parent))
                if !exact && self.solver.adts[*layout].parent == Some(*parent) =>
            {
                Ok(())
            }
            (Ty::Skolem(pid), Ty::Pacts(pids)) if !exact && pids.iter().all(|p| p == pid) => Ok(()),
            (Ty::Adt(aid), Ty::Pacts(pids))
                if !exact
                    && pids.iter().all(|pid| {
                        let aid = self.solver.adts[*aid].parent.unwrap_or(*aid);
                        self.solver.pact_impls.contains(&(*pid, aid))
                    }) =>
            {
                Ok(())
            }
            (Ty::Pacts(found), Ty::Pacts(expected))
                if !exact && expected.iter().all(|pid| found.contains(pid)) =>
            {
                Ok(())
            }
            (Ty::Fn(a), Ty::Fn(b)) if a.parameters.len() == b.parameters.len() => {
                for (a, b) in a.parameters.iter().zip(&b.parameters) {
                    if !exact && b.has_default && !a.has_default {
                        return Err(error);
                    }
                    self.relate(&b.ty, &a.ty, exact)
                        .map_err(|_| error.clone())?;
                }
                self.relate(&a.return_ty, &b.return_ty, exact)
                    .map_err(|_| error)
            }
            (Ty::Array(a), Ty::Array(b)) | (Ty::Dict(a), Ty::Dict(b)) => {
                self.relate(a, b, true).map_err(|_| error)
            }
            (Ty::Tuple(a), Ty::Tuple(b)) if a.len() == b.len() => {
                for (a, b) in a.iter().zip(b) {
                    self.relate(a, b, true).map_err(|_| error.clone())?;
                }
                Ok(())
            }
            (Ty::Option(a), Ty::Option(b)) | (Ty::Result(a), Ty::Result(b)) => {
                self.relate(a, b, exact).map_err(|_| error)
            }
            (Ty::Null, Ty::Option(_)) if !exact => Ok(()),
            (ty, Ty::Option(inner) | Ty::Result(inner)) if !exact => self.relate(ty, inner, false),
            _ => Err(error),
        }
    }
}

#[must_use]
pub(crate) struct Substitution(IndexMap<Vid, Ty>);
impl Substitution {
    /// Applies all bindings from a successful trial to the solver.
    pub(crate) fn commit(self, solver: &mut Solver) {
        for (vid, ty) in self.0 {
            #[cfg(feature = "logging")]
            println!("{}", crate::utils::Printer::substitution(&vid, &ty));
            solver.subs[vid] = Some(ty);
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnificationError {
    found: String,
    expected: String,
}
impl UnificationError {
    /// Builds an error for a binding that would make a type contain itself.
    pub(crate) fn recursive(vid: Vid, ty: &Ty) -> Self {
        Self {
            found: ty.to_string(),
            expected: format!(
                "a non-recursive type (`{}` occurs within it)",
                crate::utils::Printer::vid(&vid)
            ),
        }
    }

    /// Turns the relation error into a type mismatch diagnostic at `location`.
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
