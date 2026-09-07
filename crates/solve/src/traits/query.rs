use crate::{
    Result,
    components::{Ty, TyExt},
    errors::InvalidHex,
    *,
};
use parse::*;
use shared::Located;

use super::Solve;

pub trait Query: Located {
    /// Enforces this item to fulfill the provided Ty, registering whatever subtitutions are
    /// discovered along the way.
    fn fulfill_ty(&self, ty: Ty, solver: &mut Solver) -> Result<()> {
        // query mutates subs, so normalize `ty` afterward -- otherwise a Vid->Vid chain from query
        // can cycle on commit. `found` normalizes too: a bound vid left raw would hit unify's
        // vid arm and re-unify inside register_sub with the found/expected roles reversed
        // (the cross-module-const inverted-mismatch bug).
        let mut found = self.query(solver)?.normalized(solver);
        let mut ty = ty.normalized(solver);
        Unification::unify(&mut found, &mut ty, solver)
            .and_then(|v| v.commit(solver))
            .map_err(|e| e.into_type_mismatch(solver, self.location()))
    }

    /// Returns the Ty for this item normalized with all substitutions the solver has
    /// currently found. If this expression has never been visited, it is first solved.
    fn query(&self, solver: &mut Solver) -> Result<Ty>;
}

impl Query for Expr {
    /// Returns the Ty for this expression normalized with all substitutions the solver has
    /// currently found. If this expression has never been visited, it is first solved.
    fn query(&self, solver: &mut Solver) -> Result<Ty> {
        /// Returns the inner member of this Expr. Placeholder for moving everything to a more
        /// dyn API.
        fn inner(expr: &Expr) -> &dyn Solve {
            match expr.kind() {
                ExprKind::Absolve(v) => v,
                ExprKind::Access(v) => v,
                ExprKind::Block(v) => v,
                ExprKind::Break(v) => v,
                ExprKind::Call(v) => v,
                ExprKind::Closure(v) => v,
                ExprKind::Coalescence(v) => v,
                ExprKind::Collect(v) => v,
                ExprKind::Continue(v) => v,
                ExprKind::Equality(v) => v,
                ExprKind::Evaluation(v) => v,
                ExprKind::For(v) => v,
                ExprKind::FString(v) => v,
                ExprKind::Grouping(v) => v,
                ExprKind::Ident(v) => v,
                ExprKind::If(v) => v,
                ExprKind::In(v) => v,
                ExprKind::Literal(v) => v,
                ExprKind::Logical(v) => v,
                ExprKind::Loop(v) => v,
                ExprKind::Match(v) => v,
                ExprKind::Raise(v) => v,
                ExprKind::Range(v) => v,
                ExprKind::Return(v) => v,
                ExprKind::Unary(v) => v,
                ExprKind::Unwrap(v) => v,
                ExprKind::While(v) => v,
            }
        }

        match self.kind() {
            ExprKind::Literal(Literal::True) | ExprKind::Literal(Literal::False) => Ok(Ty::Bool),
            ExprKind::Literal(Literal::Null) => Ok(Ty::Null),
            ExprKind::Literal(Literal::Unit) => Ok(Ty::Unit),
            ExprKind::Literal(Literal::String(_)) => Ok(Ty::Str),
            ExprKind::Literal(Literal::Int(_)) => Ok(Ty::Int),
            ExprKind::Literal(Literal::Hex(h)) => {
                if i64::from_str_radix(h, 16).is_err() {
                    Err(InvalidHex {
                        src: solver.src(self.location()),
                        at: self.location().into(),
                    })?
                }
                Ok(Ty::Int)
            }
            ExprKind::Literal(Literal::Float(_)) => Ok(Ty::Float),
            _ => {
                if solver.touch_node(self.id()) {
                    #[cfg(feature = "logging")]
                    println!("{}", utils::Printer::query(self));

                    inner(self)
                        .solve(self.id(), self.location(), solver)
                        .and_then(|ty| {
                            let vid = solver.node_vid(self.id());
                            solver
                                .register_sub(vid, ty)
                                .map_err(|e| e.into_type_mismatch(solver, self.location()))
                        })?;
                }

                Ok(Ty::Vid(solver.node_vid(self.id())).normalized(solver))
            }
        }
    }
}

impl Query for Ident {
    fn query(&self, solver: &mut Solver) -> Result<Ty> {
        solver.resolve_name(self, self.location())
    }
}
