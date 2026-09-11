use crate::{
    Error, FnRun, LoopRun, Result, Solver, Unification, array,
    components::{
        Adt, AdtFlags, AdtId, DecKind, Field, Flow, FnHeader, FnParam, Quantification, Ty, TyExt,
        Variant,
    },
    errors::*,
    option,
    traits::Query,
};
use parse::{components::Binding, *};
use shared::{Located, Location};
use std::collections::HashMap;

/// Pick which native impl on `aid` named `name` dispatches for a receiver typed as `actual`.
///
/// Returns the unique matching candidate, or `None` if there's no method at all. With a single
/// candidate we skip the work and return it directly so non-overloaded methods keep their existing
/// behavior. With multiple, we freshen each candidate's recv pattern and dry-unify it against the
/// actual receiver: the substitution is computed but discarded (never `commit`ted), so testing a
/// loser only leaks unused vids. The unique success wins; zero matches surface as a type-mismatch
/// against the first candidate, multiple as ambiguity.
fn pick_method_overload(
    solver: &mut Solver,
    aid: AdtId,
    name: &str,
    actual: &Ty,
    location: Location,
) -> Result<Option<Field>> {
    let primary = solver.adts[aid].impls.get(name).cloned();
    let overloads = solver.adts[aid]
        .native_overloads
        .get(name)
        .cloned()
        .unwrap_or_default();
    let mut candidates: Vec<Field> = primary.into_iter().chain(overloads).collect();
    if candidates.len() <= 1 {
        return Ok(candidates.pop());
    }

    let actual_norm = actual.clone().normalized(solver);
    let mut matches: Vec<Field> = Vec::new();
    let mut first_pat_str = String::new();
    for cand in &candidates {
        let recv_pat = solver
            .dec_to_native
            .get(&cand.dec)
            .and_then(|b| b.sig.recv.clone());
        // shouldn't happen for the overload path (only natives go through native_overloads), but
        // if a non-native or recv-less candidate sneaks in we have no recv to test, so keep it.
        let Some(recv_pat) = recv_pat else {
            matches.push(cand.clone());
            continue;
        };
        let mut memo = HashMap::new();
        let mut pat = solver.substitute_anons(&recv_pat, &mut memo);
        if first_pat_str.is_empty() {
            first_pat_str = pat.to_string();
        }
        let mut actual_clone = actual_norm.clone();
        if Unification::unify(&mut actual_clone, &mut pat, &*solver).is_ok() {
            matches.push(cand.clone());
        }
    }

    match matches.len() {
        0 => Err(TypeMismatch {
            src: solver.src(location),
            at: location.into(),
            expected: first_pat_str,
            found: actual_norm.to_string(),
        }
        .into()),
        1 => Ok(Some(matches.into_iter().next().unwrap())),
        _ => Err(AmbiguousPactMember {
            src: solver.src(location),
            at: location.into(),
            member: name.to_string(),
        }
        .into()),
    }
}

pub trait Solve {
    /// Evaluates this T and its inner nodes on the provided Session to discover its Ty.
    fn solve(&self, id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty>;
}

impl Solve for Access {
    fn solve(&self, id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        fn handle_adt_access(
            id: NodeId,
            solver: &mut Solver,
            left: &Expr,
            right: &Ident,
        ) -> Result<Ty> {
            let aid = adt_from_type_path(left, solver)?;
            let adt = &solver.adts[aid];

            // variants live in `adt.variants`, not `adt.impls` -- try them first. an empty variant
            // is an instance value at the access site; a non-empty one is only accessible through a
            // constructor (`Foo::Bar(..)` or `Foo::Bar { .. }`) handled in Call / Literal::Struct.
            if let Some(variant) = adt.variants.get(&right.lexeme) {
                return if variant.is_empty() {
                    // resolve to the variant's first-class dec: the IR matches on
                    // `DeclKind::Variant { layout, .. }` to know what struct_id to build.
                    let dec = variant
                        .dec()
                        .expect("enum variant must have a dec set during hoist");
                    solver.node_decs.insert(id, dec);
                    Ok(Ty::Adt(aid))
                } else {
                    Err(MissingTupleMembers {
                        src: solver.src(right.location()),
                        at: right.location().into(),
                    })?
                };
            }

            // modules expose their items as fields, so fall back to fields when the ADT was
            // declared as a module
            let field = adt
                .impls
                .get(&right.lexeme)
                .or_else(|| {
                    if adt.flags.contains(AdtFlags::IS_MODULE) {
                        adt.as_struct().fields.get(&right.lexeme)
                    } else {
                        None
                    }
                })
                .cloned()
                .ok_or_else(|| {
                    Error::from(FieldNotFound {
                        src: solver.src(right.location()),
                        at: right.location().into(),
                        field_name: right.lexeme.clone(),
                    })
                })?;

            let dec = field.dec;
            solver.check_vis(dec, right.location())?;
            solver.node_decs.insert(id, dec);
            // module-nested natives need fresh type vars per call site, same as free natives.
            if let Some(binding) = solver.dec_to_native.get(&dec) {
                let sig = crate::NativeFnSig {
                    params: binding.sig.params.clone(),
                    return_ty: binding.sig.return_ty.clone(),
                    recv: binding.sig.recv.clone(),
                };
                return solver.instantiate_native(&sig, None);
            }

            Ok(field.ty)
        }
        // can a plain `[i]` / `.x` ride the short-circuit on `e`, or does `e` leave a *fresh*
        // option that needs its own `?`? the mental model is "one `?` per option you reach
        // through": a `?` rides cleanly through accesses that introduce no new option
        // (array index, field), but a dict index is a fresh option (the key might miss), so
        // the next access spends its own `?`. that's why `f["x"]?["y"][0]` is rejected --
        // the `["y"]` miss isn't acknowledged -- while `a[0]["foo"]?[0][0]` is fine, the
        // trailing accesses being array drills. a dict never rides, even as `?["y"]`: the
        // `?` consumes the *incoming* option but the key's own miss is still unhandled. a
        // plain index of a fresh option just falls through to the normal path below and
        // fails to unify (option vs array/dict), so no dedicated error is needed. only the solver
        // needs this; codegen still null-checks anything `?`-tainted, which stays correct for what
        // the solver lets through.
        fn rides(solver: &mut Solver, e: &Expr) -> bool {
            match e.kind() {
                ExprKind::Access(Access::Square { left, key, kind }) => {
                    let dict = key
                        .query(solver)
                        .is_ok_and(|t| matches!(t.normalized(solver), Ty::Str));
                    !dict && (*kind == AccessKind::Option || rides(solver, left))
                }
                ExprKind::Access(Access::Dot { left, kind, .. }) => {
                    *kind == AccessKind::Option || rides(solver, left)
                }
                _ => false,
            }
        }
        match self {
            Access::Square { left, key, kind } => {
                let k = key.query(solver)?;
                let key_src = solver.src(location);

                // strings index by char position and never unify against the array/dict shape
                // below; a non-int key gets its own error so the shape's fresh vid never leaks
                // into a user-facing message
                let receiver = left.query(solver)?.normalized(solver);
                let optional = *kind == AccessKind::Option || rides(solver, left);
                if receiver == Ty::Str || (optional && receiver == option!(Ty::Str)) {
                    if k.normalized(solver) != Ty::Int {
                        return Err(StringIndexing {
                            src: solver.src(key.location()),
                            at: key.location().into(),
                        }
                        .into());
                    }
                    return Ok(if optional { option!(Ty::Str) } else { Ty::Str });
                }

                let shape = |inner: Ty| -> Result<Ty> {
                    match &k {
                        Ty::Int => Ok(Ty::Array(Box::new(inner))),
                        Ty::Str => Ok(Ty::Dict(Box::new(inner))),
                        _ => Err(InvalidAccess {
                            src: key_src.clone(),
                            at: location.into(),
                            ty: k.to_string(),
                        })?,
                    }
                };
                if *kind == AccessKind::Option || rides(solver, left) {
                    let dummy_var = solver.vid();
                    let ty = option!(shape(Ty::Vid(dummy_var))?);
                    left.fulfill_ty(ty, solver)?;
                    Ok(option!(Ty::Vid(dummy_var).normalized(solver)))
                } else {
                    let value = solver.vid();
                    let ty = shape(Ty::from(value))?;
                    let is_dict = matches!(ty, Ty::Dict(_));
                    left.fulfill_ty(ty, solver)?;

                    let out = if is_dict {
                        option!(Ty::from(value))
                    } else {
                        Ty::from(value)
                    };

                    Ok(out)
                }
            }
            Access::Dot { left, right, kind } => {
                let opt = kind == &AccessKind::Option || rides(solver, left);
                let lhs_outer = left.query(solver)?;
                let lhs = if opt {
                    match lhs_outer {
                        Ty::Option(inner) => inner.as_ref().clone(),
                        ref l => Err(InvalidUnwrap {
                            src: solver.src(left.location()),
                            at: left.location().into(),
                            ty: l.to_string(),
                        })?,
                    }
                } else {
                    lhs_outer
                };

                let result = match &lhs {
                    Ty::Tuple(members) => {
                        let ExprKind::Literal(Literal::Int(i)) = right.kind() else {
                            Err(InvalidTupleIndex {
                                src: solver.src(right.location()),
                                at: right.location().into(),
                            })?
                        };
                        members.get(*i as usize).cloned().ok_or_else(|| {
                            Error::from(OutOfBoundsTupleRead {
                                src: solver.src(location),
                                at: location.into(),
                                tried_index: *i as usize,
                                max_index: members.len(),
                                ty: Ty::Tuple(members.clone()).to_string(),
                            })
                        })?
                    }
                    Ty::Pacts(pids) => {
                        if let ExprKind::Literal(Literal::Int(_)) = right.kind() {
                            return Err(NotATuple {
                                src: solver.src(location),
                                at: location.into(),
                            }
                            .into());
                        }

                        // saftey: parser rejects any dot that is not ident or int
                        let name = &right.as_ident().unwrap().lexeme;
                        let mut con = None;
                        let mut fun = None;
                        let mut matches = 0;
                        for pid in pids {
                            let pact = &solver.pacts[pid];
                            if let Some(c) = pact.constants.get(name) {
                                con = Some(c.clone());
                                matches += 1;
                            } else if let Some((header, _)) = pact.functions.get(name) {
                                fun = Some(header.clone());
                                matches += 1;
                            }
                        }

                        if matches > 1 {
                            return Err(AmbiguousPactMember {
                                src: solver.src(right.location()),
                                at: right.location().into(),
                                member: name.clone(),
                            }
                            .into());
                        }
                        if con.is_some() {
                            // methods dispatch through `pact_impls`, but constants have no
                            // runtime dispatch -- every impl declares its own value and the
                            // receiver's concrete type isn't known here
                            return Err(PactConstantNotDispatchable {
                                src: solver.src(right.location()),
                                at: right.location().into(),
                                member: name.clone(),
                                via: lhs.to_string(),
                            }
                            .into());
                        } else if let Some(header) = fun {
                            // freshen so dispatch can't bind the pact header's shared self-slot
                            // vid, which would pollute every later use
                            // and impl signature check.
                            solver.instantiate_pact_fn(&header)
                        } else {
                            return Err(FieldNotFound {
                                src: solver.src(location),
                                at: location.into(),
                                field_name: name.clone(),
                            }
                            .into());
                        }
                    }
                    other => {
                        // adts/identity resolve directly; primitives & collections route through
                        // their library adt so builtin methods and bare method refs resolve like
                        // any other impl.
                        let aid = match other {
                            Ty::Adt(aid) | Ty::Identity(aid) => *aid,
                            l => solver.builtin_adt(l).ok_or_else(|| {
                                Error::from(TypeHasNoFields {
                                    src: solver.src(left.location()),
                                    at: left.location().into(),
                                    ty: l.to_string(),
                                })
                            })?,
                        };

                        if let ExprKind::Literal(Literal::Int(i)) = right.kind() {
                            let i = *i as usize;
                            let adt = &solver.adts[aid];
                            let tup_info = match adt.variants.values().next() {
                                Some(Variant::Tuple(t)) => Some((
                                    t.members.get(i).cloned(),
                                    t.members.len(),
                                    adt.name.clone(),
                                )),
                                _ => None,
                            };
                            match tup_info {
                                Some((Some(t), _, _)) => t,
                                Some((None, max, name)) => Err(OutOfBoundsTupleRead {
                                    src: solver.src(location),
                                    at: location.into(),
                                    tried_index: i,
                                    max_index: max,
                                    ty: name,
                                })?,
                                None => Err(NotATuple {
                                    src: solver.src(location),
                                    at: location.into(),
                                })?,
                            }
                        } else {
                            // saftey: parser rejects any dot that is not ident or int
                            let name = &right.as_ident().unwrap().lexeme;

                            // fields win over impls. method dispatch happens @ call
                            let field_ty = match solver.adts[aid].variants.values().next() {
                                Some(Variant::Struct(s)) => {
                                    s.fields.get(name).map(|f| f.ty.clone())
                                }
                                _ => None,
                            };

                            if let Some(ty) = field_ty {
                                ty
                            } else {
                                let impl_field = pick_method_overload(
                                    solver,
                                    aid,
                                    name,
                                    &lhs,
                                    right.location(),
                                )?;
                                let Some(impl_field) = impl_field else {
                                    Err(FieldNotFound {
                                        src: solver.src(location),
                                        at: location.into(),
                                        field_name: name.clone(),
                                    })?
                                };
                                let dec = impl_field.dec;
                                solver.check_vis(dec, right.location())?;
                                solver.node_decs.insert(id, dec);
                                if let Some(binding) = solver.dec_to_native.get(&dec) {
                                    let sig = crate::NativeFnSig {
                                        params: binding.sig.params.clone(),
                                        return_ty: binding.sig.return_ty.clone(),
                                        recv: binding.sig.recv.clone(),
                                    };
                                    solver
                                        .instantiate_native(&sig, Some((&lhs, left.location())))?
                                } else {
                                    impl_field.ty
                                }
                            }
                        }
                    }
                };

                if opt && !matches!(result, Ty::Fn(_)) {
                    Ok(Ty::Option(Box::new(result)))
                } else {
                    Ok(result)
                }
            }

            Access::Identity { .. } => {
                unreachable!("Access::Identity is never produced by the parser")
            }
            Access::DoubleColon { left, right } => {
                // `Self::member` inside a pact default resolves against the contextual pact.
                // There's no explicit `Pact::member` form -- concrete access goes
                // through the adt name.
                if let ExprKind::Ident(ident) = left.kind()
                    && ident.is_identity()
                    && let Some(pid) = solver.pact_self
                {
                    let pact = &solver.pacts[pid];
                    if pact.constants.contains_key(&right.lexeme) {
                        // a default body is compiled once and shared by every impl, so `Self`
                        // is still the abstract pact here and there's no single constant to
                        // point at -- same limitation as reaching one through a value
                        Err(PactConstantNotDispatchable {
                            src: solver.src(right.location),
                            at: right.location.into(),
                            member: right.lexeme.clone(),
                            via: "Self".into(),
                        }
                        .into())
                    } else if let Some((header, _)) = pact.functions.get(&right.lexeme) {
                        let header = header.clone();
                        Ok(solver.instantiate_pact_fn(&header))
                    } else {
                        Err(FieldNotFound {
                            src: solver.src(right.location),
                            at: right.location.into(),
                            field_name: right.lexeme.clone(),
                        }
                        .into())
                    }
                } else {
                    handle_adt_access(id, solver, left, right)
                }
            }
        }
    }
}

impl Solve for Block {
    fn solve(&self, _id: NodeId, _location: Location, solver: &mut Solver) -> Result<Ty> {
        solver.ribs.push_block();
        solver.hoist_block_consts(&self.body);
        let mut diverged = false;
        for stmt in &self.body {
            solver.visit_stmt(stmt)?;
            if let StmtKind::Expr(e) = stmt.kind()
                && matches!(e.query(solver)?, Ty::Never)
            {
                diverged = true;
            }
        }
        let yielded = self
            .yielded_expr
            .as_ref()
            .map(|v| v.query(solver))
            .transpose()?;
        let ty = if diverged {
            Ty::Never
        } else {
            yielded.unwrap_or(Ty::Unit)
        };
        solver.ribs.pop();
        Ok(ty)
    }
}

impl Solve for Break {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        let ty = self
            .value
            .as_ref()
            .map_or(Ok(Ty::Unit), |v| v.query(solver))?;
        let src = solver.src(location);
        solver.with_loop_mut(
            || {
                BreakOutsideLoop {
                    src: src.clone(),
                    at: location.into(),
                }
                .into()
            },
            |solver, loop_data| {
                solver
                    .regsiter_loop_control_flow_ty(ty, &mut loop_data.break_ty)
                    .map_err(|e| e.into_type_mismatch(solver, location))
            },
        )?;
        Ok(Ty::Never)
    }
}

impl Solve for Call {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        let adt = match self.left.kind() {
            ExprKind::Access(Access::DoubleColon { left, right }) => {
                let adt = adt_from_type_path(left, solver).map_err(|_| {
                    let ty = left
                        .query(solver)
                        .map(|t| t.to_string())
                        .unwrap_or_default();
                    NotAStruct {
                        src: solver.src(location),
                        at: location.into(),
                        ty,
                    }
                })?;

                let lock = &solver.adts[adt];
                if lock.flags.contains(AdtFlags::IS_MODULE) {
                    // field.ty is the bare vid for a module item, so normalize to chase it to the
                    // actual adt id before we treat it as a constructor target.
                    let field_ty = lock
                        .as_struct()
                        .fields
                        .get(&right.lexeme)
                        .map(|f| f.ty.clone());
                    field_ty.and_then(|ty| {
                        let layout = *ty.normalized(solver).as_adt()?;
                        Some((
                            layout,
                            solver.adts[layout].as_singular().unwrap().clone(),
                            layout,
                        ))
                    })
                } else {
                    lock.variants
                        .get(&right.lexeme)
                        .cloned()
                        .map(|v| (v.layout().unwrap(), v, adt))
                }
            }
            ExprKind::Ident(ident) => match ident.query(solver)? {
                Ty::Adt(aid) | Ty::Identity(aid) => {
                    let (is_enum, name, singular) = {
                        let adt = &solver.adts[aid];
                        (
                            adt.flags.contains(AdtFlags::IS_ENUM),
                            adt.name.clone(),
                            adt.as_singular().cloned(),
                        )
                    };
                    if is_enum {
                        Err(EnumNotConstructable {
                            src: solver.src(location),
                            at: location.into(),
                            name,
                        })?
                    }
                    singular.map(|v| (aid, v, aid))
                }
                _ => None,
            },
            _ => None,
        };

        if let Some((layout, variant, adt)) = adt {
            match variant {
                Variant::Tuple(variant) => {
                    if let Some(named) = self.arguments.iter().find(|a| a.name.is_some()) {
                        let name = named.name.as_ref().unwrap();
                        Err(UnknownNamedArgument {
                            src: solver.src(name.location),
                            at: name.location.into(),
                            name: name.lexeme.clone(),
                        })?
                    }
                    let mut args = self.arguments.clone().into_iter().peekable();
                    let mut params = variant.members.clone().into_iter();

                    loop {
                        match (args.next(), params.next()) {
                            (Some(arg), Some(param)) => arg.value.fulfill_ty(param, solver)?,
                            (Some(_), None) => Err(ExtraTupleMembers {
                                src: solver.src(location),
                                at: location.into(),
                            })?,
                            (None, Some(_)) => Err(MissingTupleMembers {
                                src: solver.src(location),
                                at: location.into(),
                            })?,
                            _ => break,
                        }
                    }

                    solver.shadow_expr_ty(self.left.id(), Ty::Adt(layout), self.left.location())?;
                    Ok(Ty::Adt(adt))
                }
                v @ Variant::Struct(_) => Err(ExpectedTupleStruct {
                    src: solver.src(location),
                    at: location.into(),
                    ty: v.to_string(),
                })?,
            }
        } else {
            // methods win over same-named fields at call sites -- Access::Dot is field-first
            // for value reads, so short-circuit here to make the builder pattern work
            // (`s.power(5)` where `power` is also a field).
            fn method_intercept(call: &Call, solver: &mut Solver) -> Result<Option<Ty>> {
                let ExprKind::Access(Access::Dot { left, right, kind }) = call.left.kind() else {
                    return Ok(None);
                };
                let Some(ident) = right.as_ident() else {
                    return Ok(None);
                };
                let lhs = match (kind, left.query(solver)?.normalized(solver)) {
                    (AccessKind::Option, Ty::Option(inner)) => *inner,
                    (AccessKind::Option, other) => Err(InvalidUnwrap {
                        src: solver.src(left.location()),
                        at: left.location().into(),
                        ty: other.to_string(),
                    })?,
                    (_, other) => other,
                };
                let aid = match &lhs {
                    Ty::Adt(a) | Ty::Identity(a) => *a,
                    l => match solver.builtin_adt(l) {
                        Some(a) => a,
                        None => return Ok(None),
                    },
                };
                let Some(field) =
                    pick_method_overload(solver, aid, &ident.lexeme, &lhs, ident.location)?
                else {
                    return Ok(None);
                };
                let dec = field.dec;
                solver.check_vis(dec, ident.location)?;
                solver.node_decs.insert(call.left.id(), dec);
                let ty = if let Some(binding) = solver.dec_to_native.get(&dec) {
                    let sig = crate::NativeFnSig {
                        params: binding.sig.params.clone(),
                        return_ty: binding.sig.return_ty.clone(),
                        recv: binding.sig.recv.clone(),
                    };
                    solver.instantiate_native(&sig, Some((&lhs, left.location())))?
                } else {
                    field.ty
                };
                Ok(Some(ty))
            }
            let ty = match method_intercept(self, solver)? {
                Some(t) => {
                    solver.shadow_expr_ty(self.left.id(), t.clone(), self.left.location())?;
                    t.normalized(solver)
                }
                None => self.left.query(solver)?.normalized(solver),
            };
            let fn_data = match ty {
                Ty::Fn(fn_data) => fn_data,
                // unbound callee (e.g. `fn apply(f, x) { f(x) }` -- `f` has no annotation, so
                // its type is still an unresolved vid when we solve the body). synthesize a
                // fn shape with fresh vids for each arg + return and unify it back onto the
                // callee. cascading constraints fill the params/return when the outer call
                // (`apply(double, 5)`) supplies a concrete `double`.
                //
                // named args aren't supported in this path: the synthesized params have no
                // source names. anything that tried `f(b=1)` falls through to NotCallable
                // since we can't decide what `b` refers to without a real signature.
                Ty::Vid(_) if self.arguments.iter().all(|a| a.name.is_none()) => {
                    let synth_params: Vec<FnParam> = self
                        .arguments
                        .iter()
                        .map(|_| FnParam::new(None, Ty::Vid(solver.vid()), false))
                        .collect();
                    let synth_return = Ty::Vid(solver.vid());
                    let synth =
                        FnHeader::new(synth_params, synth_return, /* is_method */ false);
                    self.left.fulfill_ty(Ty::Fn(synth.clone()), solver)?;
                    synth
                }
                _ => Err(NotCallable {
                    src: solver.src(location),
                    at: location.into(),
                    ty: ty.to_string(),
                })?,
            };
            let params: Vec<FnParam> = fn_data.parameters;
            let mut filled = vec![false; params.len()];
            let mut cursor = 0usize;

            if fn_data.is_method
                && let ExprKind::Access(Access::Dot { left, kind, .. }) = self.left.kind()
                && let Some(first) = params.first()
            {
                // a `?.` receiver is option-typed; dispatch is on the unwrapped inner
                // (method_intercept already vetted it), so expect Option(self) here
                let expected = if *kind == AccessKind::Option {
                    Ty::Option(Box::new(first.ty.clone()))
                } else {
                    first.ty.clone()
                };
                left.fulfill_ty(expected, solver)?;
                filled[0] = true;
                cursor = 1;
            }

            for arg in &self.arguments {
                let slot = match &arg.name {
                    None => {
                        if cursor >= params.len() {
                            Err(ExtraArguments {
                                src: solver.src(location),
                                at: location.into(),
                            })?
                        }
                        let s = cursor;
                        cursor += 1;
                        s
                    }
                    Some(name) => params
                        .iter()
                        .position(|p| p.name.as_deref() == Some(name.lexeme.as_str()))
                        .ok_or_else(|| UnknownNamedArgument {
                            src: solver.src(name.location),
                            at: name.location.into(),
                            name: name.lexeme.clone(),
                        })?,
                };
                if filled[slot] {
                    let name = arg.name.as_ref().unwrap();
                    Err(DuplicateNamedArgument {
                        src: solver.src(name.location),
                        at: name.location.into(),
                        name: name.lexeme.clone(),
                    })?
                }
                if let ExprKind::Closure(_) = arg.value.kind()
                    && let Ty::Fn(header) = params[slot].ty.clone().normalized(solver)
                {
                    solver
                        .expected_closures
                        .insert(arg.value.id(), header.parameters);
                }
                arg.value.fulfill_ty(params[slot].ty.clone(), solver)?;
                filled[slot] = true;
            }
            if params
                .iter()
                .zip(&filled)
                .any(|(p, &f)| !f && !p.has_default)
            {
                Err(MissingArguments {
                    src: solver.src(location),
                    at: location.into(),
                })?;
            }

            let ty = fn_data.return_ty.as_ref().clone().normalized(solver);
            if let ExprKind::Access(Access::Dot {
                kind: AccessKind::Option,
                ..
            }) = self.left.kind()
            {
                Ok(Ty::Option(Box::new(ty)))
            } else {
                Ok(ty)
            }
        }
    }
}

impl Solve for Closure {
    fn solve(&self, id: NodeId, _location: Location, solver: &mut Solver) -> Result<Ty> {
        solver.ribs.push_closure(id);

        // a call site may have stashed the fn-typed parameter it expects here -- seed each closure
        // param's vid with it before the body solves, so `|n| n + 1` learns `n: int` from the
        // annotation rather than erroring on an unresolved vid. only when arities match; a mismatch
        // falls through to the call-site unification, which reports it.
        let expected = solver
            .expected_closures
            .swap_remove(&id)
            .filter(|e| e.len() == self.parameters.len());

        let mut parameters = vec![];
        for (i, binding) in self.parameters.iter().enumerate() {
            let mut param = solver.bind_parameter(binding)?;
            if let Some(expected) = &expected {
                let mut want = expected[i].ty.clone();
                param
                    .ty
                    .fulfill_ty(&mut want, solver)
                    .map_err(|e| e.into_type_mismatch(solver, binding.location()))?;
                param.ty = param.ty.normalized(solver);
            }
            // closures have no hoist phase, so the real per-body declare happens here. mirrors
            // the loop in `Function::solve`. `bind_parameter` itself no longer declares -- it
            // just resolves the param's type into an `FnParam`.
            solver.declare(
                binding.left.as_ident().unwrap(),
                binding.left.id(),
                param.ty.clone(),
            )?;
            parameters.push(param);
        }

        let body_ty = self.body.query(solver)?;
        solver.ribs.pop();

        Ok(Ty::Fn(FnHeader::new(parameters, body_ty, false)))
    }
}

impl Solve for Coalescence {
    fn solve(&self, _id: NodeId, _location: Location, solver: &mut Solver) -> Result<Ty> {
        let lhs = self.left.query(solver)?.normalized(solver);
        match lhs {
            Ty::Option(inner) => {
                let inner = *inner;
                self.right.fulfill_ty(inner.clone(), solver)?;
                Ok(inner)
            }
            Ty::Null => self.right.query(solver),
            other => Err(CoalesceNonOptional {
                src: solver.src(self.left.location()),
                at: self.left.location().into(),
                ty: other.to_string(),
            })?,
        }
    }
}

impl Solve for Collect {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        let ty = self.value.query(solver)?;
        let src = solver.src(location);
        solver.with_loop_mut(
            || {
                CollectOutsideLoop {
                    src: src.clone(),
                    at: location.into(),
                }
                .into()
            },
            |solver, loop_data| {
                solver
                    .regsiter_loop_control_flow_ty(ty, &mut loop_data.collect_ty)
                    .map_err(|e| e.into_type_mismatch(solver, location))
            },
        )?;
        Ok(Ty::Never)
    }
}

impl Solve for Continue {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        if solver.loop_stack.last().is_none() {
            Err(ContinueOutOfLoop {
                src: solver.src(location),
                at: location.into(),
            })?
        }
        Ok(Ty::Never)
    }
}

impl Solve for Enum {
    fn solve(&self, _id: NodeId, _location: Location, solver: &mut Solver) -> Result<Ty> {
        let Ty::Adt(adt) = self.head.query(solver)? else {
            unreachable!()
        };

        // self-referential variants (`enum List { Cons(int, List), Nil }`) need filter_adt to
        // break the recursion check on field types -- same pattern as Struct::solve.
        solver.ribs.push_impl(adt);

        let result = self
            .members
            .iter()
            .try_for_each(|(member_name, member)| match member {
                Member::Tuple(members) => {
                    members.iter().enumerate().try_for_each(|(i, annotation)| {
                        let mut ty = Ty::from_annotation(annotation.clone(), solver)?;
                        let mut field_ty = solver.adts[adt]
                            .variants
                            .get_mut(&member_name.lexeme)
                            .unwrap()
                            .as_tuple()
                            .members
                            .get(i)
                            .unwrap()
                            .clone();

                        field_ty
                            .fulfill_ty(&mut ty, solver)
                            .map_err(|v| v.into_type_mismatch(solver, member_name.location()))
                    })
                }
                Member::Struct(fields) => fields.iter().try_for_each(|field| {
                    let mut ty = Ty::from_annotation(field.annotation.clone(), solver)?;
                    let mut field_ty = solver.adts[adt]
                        .variants
                        .get_mut(&member_name.lexeme)
                        .unwrap()
                        .as_struct()
                        .ty(&field.name.to_string())
                        .unwrap();
                    field_ty
                        .fulfill_ty(&mut ty, solver)
                        .map_err(|v| v.into_type_mismatch(solver, field.location))
                }),
            });

        solver.ribs.pop();
        result?;

        // todo: helper method on solver to avoid this clone
        let mut normalized = solver.adts[adt].clone();
        normalized.normalize_members(solver);
        let layouts: Vec<_> = normalized
            .variants
            .values()
            .filter_map(|variant| {
                variant.layout().map(|layout| {
                    let name = solver.adts[layout].name.clone();
                    (layout, Adt::new_variant_layout(name, variant))
                })
            })
            .collect();
        solver.adts[adt] = normalized;
        for (layout, adt) in layouts {
            solver.adts[layout] = adt;
        }

        Ok(Ty::Adt(adt))
    }
}

impl Solve for Equality {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        let lhs = self.left.query(solver)?;
        let rhs = self.right.query(solver)?;
        match (&lhs, &rhs) {
            (Ty::Null, Ty::Option(_)) | (Ty::Option(_), Ty::Null) => Ok(()),
            (l, r) if l.is_numeric() && r.is_numeric() => Ok(()),
            _ if matches!(self.op, EqualityOp::Equal | EqualityOp::NotEqual) => {
                // the LHS-driven rule lets `T? == T` coerce; mirror it when only the rhs
                // is optional so `T == T?` is equally legal (compare-inner -- null vs a
                // value is just false). the mirror needs a settled non-option lhs, so
                // inference for unresolved operands keeps pinning rhs to lhs as before.
                let l = lhs.clone().normalized(solver);
                let r = rhs.clone().normalized(solver);
                if matches!(r, Ty::Option(_)) && !matches!(l, Ty::Option(_) | Ty::Vid(_)) {
                    self.left.fulfill_ty(self.right.query(solver)?, solver)
                } else {
                    self.right.fulfill_ty(self.left.query(solver)?, solver)
                }
            }
            _ => Err(InvalidComparison {
                src: solver.src(location),
                at: location.into(),
            })?,
        }?;

        Ok(Ty::Bool)
    }
}

impl Solve for Evaluation {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        fn eval(
            op: EvaluationOp,
            lhs: &Ty,
            rhs: &Ty,
            location: Location,
            solver: &Solver,
        ) -> Result<Ty> {
            match (lhs, rhs) {
                (Ty::Str, Ty::Str) if op == EvaluationOp::Plus => Ok(Ty::Str),
                (Ty::Bool, Ty::Bool) if op.is_binary() => Ok(Ty::Bool),
                (Ty::Int, Ty::Int) if op == EvaluationOp::Divide => Ok(Ty::Float),
                (Ty::Int, Ty::Int) => Ok(Ty::Int),
                (Ty::Float, Ty::Float) | (Ty::Int, Ty::Float) | (Ty::Float, Ty::Int)
                    if !op.is_bitwise() =>
                {
                    Ok(Ty::Float)
                }

                (Ty::Tuple(a), Ty::Tuple(b)) if a.len() == b.len() => a
                    .iter()
                    .zip(b)
                    .map(|(a, b)| eval(op, a, b, location, solver))
                    .collect::<Result<Vec<Ty>>>()
                    .map(Ty::Tuple),

                (Ty::Tuple(elems), s) if matches!(s, Ty::Int | Ty::Float) => elems
                    .iter()
                    .map(|e| eval(op, e, s, location, solver))
                    .collect::<Result<Vec<Ty>>>()
                    .map(Ty::Tuple),

                (s, Ty::Tuple(elems)) if matches!(s, Ty::Int | Ty::Float) => elems
                    .iter()
                    .map(|e| eval(op, s, e, location, solver))
                    .collect::<Result<Vec<Ty>>>()
                    .map(Ty::Tuple),

                (lhs, rhs) => Err(InvalidEvaluation {
                    src: solver.src(location),
                    at: location.into(),
                    lhs: lhs.to_string(),
                    op: op.to_string(),
                    rhs: rhs.to_string(),
                }
                .into()),
            }
        }

        let lhs = self.left.query(solver)?;
        let rhs = self.right.query(solver)?;
        eval(self.op, &lhs, &rhs, location, solver)
    }
}

impl Solve for For {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        // Run the loop
        solver.ribs.push_block();

        let iter_ty = self.iterator.query(solver)?.normalized(solver);
        match iter_ty {
            Ty::Str => {
                solver.solve_pat(&self.binding, Ty::Str)?;
            }
            Ty::Int => {
                solver.solve_pat(&self.binding, Ty::Int)?;
            }
            Ty::Array(value) => {
                solver.solve_pat(&self.binding, value.as_ref().clone())?;
            }
            Ty::Dict(value) => {
                solver.solve_pat(&self.binding, Ty::Tuple(vec![Ty::Str, *value]))?;
            }
            other => Err(InvalidIterTarget {
                src: solver.src(self.iterator.location()),
                at: self.iterator.location().into(),
                ty: other.to_string(),
            })?,
        }

        if let Some(dec_id) = solver.node_decs.get(&self.binding.id()) {
            let dec = &mut solver.decs[dec_id];
            dec.kind = DecKind::LoopVar;
        }

        let LoopRun {
            break_ty,
            collect_ty,
            ..
        } = solver.run_loop_body(&self.body)?;
        solver.ribs.pop();

        // Validate the types
        let break_ty = break_ty.unwrap_or_default();
        Ok(match (break_ty, collect_ty) {
            (Ty::Unit, None) => Ty::Unit,
            (bty, None) => Ty::Option(Box::new(bty)),
            (Ty::Unit, Some(cty)) => array!(cty),
            (_, Some(_)) => Err(BreakValueWhileCollection {
                src: solver.src(location),
                at: location.into(),
            })?,
        })
    }
}

impl Solve for FString {
    fn solve(&self, _id: NodeId, _location: Location, solver: &mut Solver) -> Result<Ty> {
        for part in &self.parts {
            if let FStringPart::Expr(expr) = part {
                expr.query(solver)?;
            }
        }
        Ok(Ty::Str)
    }
}

impl Solve for Function {
    fn solve(&self, id: NodeId, _location: Location, solver: &mut Solver) -> Result<Ty> {
        let vid = solver.node_vid(id);

        #[cfg(feature = "logging")]
        println!("{}", crate::utils::Printer::vid(&vid));

        // hoist only seeds the vid for top-level / impl-body fns. an unseeded vid here means
        // we're a `fn` inside a fn body -- mimas has no support for that scope, only closures.
        let Some(ty) = solver.sub(vid).cloned() else {
            Err(NestedFn {
                src: solver.src(self.name.location),
                at: self.name.location.into(),
                name: self.name.lexeme.clone(),
            })?
        };
        let Ty::Fn(fn_data) = ty else {
            unreachable!("hoist seeded the vid with something other than Ty::Fn")
        };

        // one rib holds the params and doubles as the capture barrier; the body's block pushes its
        // own scope on top.
        solver.ribs.push_function();
        let param_tys = fn_data.parameters.iter();
        let parse_params = self.parameters.iter();

        for (param, Binding { left, .. }) in param_tys.zip(parse_params) {
            solver.declare(left.as_ident().unwrap(), left.id(), param.ty.clone())?;
        }

        let expected_ty = fn_data.return_ty.as_ref().clone();
        solver.fn_stack.push(FnRun {
            expected_ty: expected_ty.clone(),
        });
        solver.control_flow.enter();
        let body_ty = self.body.query(solver)?;
        solver.fn_stack.pop().unwrap();
        let flow = solver.control_flow.exit();

        if body_ty != Ty::Unit {
            self.body.fulfill_ty(expected_ty.clone(), solver)?;
        } else if expected_ty != Ty::Unit
            && solver.control_flow.quantify(&flow) != Quantification::Universal
        {
            Err(NotAllPathsReturn {
                src: solver.src(self.body.location()),
                at: self.body.location().into(),
                ty: expected_ty.to_string(),
            })?;
        }

        solver.ribs.pop();

        Ok(Ty::Fn(fn_data.clone()))
    }
}

impl Solve for Grouping {
    fn solve(&self, _id: NodeId, _location: Location, solver: &mut Solver) -> Result<Ty> {
        self.inner.query(solver)
    }
}

impl Solve for Ident {
    fn solve(&self, id: NodeId, _location: Location, solver: &mut Solver) -> Result<Ty> {
        let ty = self.query(solver)?;

        // mark this usage; every closure boundary crossed on the way to the binding captures it
        if let Some((dec_id, crossed)) = solver.ribs.resolve_with_closures(self) {
            solver.node_decs.insert(id, dec_id);
            if matches!(solver.decs[dec_id].kind, DecKind::Local | DecKind::LoopVar) {
                for closure in crossed {
                    solver
                        .closure_captures
                        .entry(closure)
                        .or_default()
                        .insert(dec_id);
                }
            }
        }

        if let Ty::Adt(adt) = &ty {
            let members = {
                let lock = &solver.adts[adt];
                (self.lexeme == lock.name && lock.is_tuple_struct())
                    .then(|| lock.as_tuple().members.clone())
            };
            if let Some(members) = members {
                let params = members
                    .into_iter()
                    .map(|m| FnParam::new(None, m.normalized(solver), false))
                    .collect();
                return Ok(Ty::Fn(FnHeader::new(params, ty, false).ctor()));
            }
        }
        Ok(ty)
    }
}

impl Solve for If {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        if let Some(binding) = self.binding.as_ref() {
            let scrut_ty = self.condition.query(solver)?.normalized(solver);
            solver.ribs.push_block();
            // `if let` over an option binds the unwrapped value (implicit null test). an explicit
            // `?` pattern does its own unwrapping, so leave the option intact for it.
            solver.solve_match_pat(binding, scrut_ty, false)?;
        } else {
            self.condition.fulfill_ty(Ty::Bool, solver)?;
        }

        solver.control_flow.enter();
        let positive_ty = self.main_body.query(solver)?;
        let positive = solver.control_flow.exit();

        // Ensure the branches match, or coercse an option.
        let (ty, negative) = if let Some(else_expr) = self.else_expr.as_ref() {
            solver.control_flow.enter();
            let ty = if let Err(e) = else_expr.fulfill_ty(positive_ty.clone(), solver) {
                if let Some(ty) = Ty::coerce_option(else_expr.query(solver)?, positive_ty, solver) {
                    ty
                } else {
                    Err(e)?
                }
            } else {
                positive_ty.clone()
            };
            (ty, solver.control_flow.exit())
        } else {
            // If there's no else then the main body must be ()
            self.main_body
                .fulfill_ty(Ty::Unit, solver)
                .map_err(|_| IfNeedsElse {
                    src: solver.src(location),
                    at: location.into(),
                })?;
            (Ty::Unit, solver.control_flow.push(Flow::Block(vec![])))
        };

        solver
            .control_flow
            .push(Flow::Potential { positive, negative });

        if self.binding.is_some() {
            solver.ribs.pop();
        }

        Ok(ty)
    }
}

impl Solve for Impl {
    fn solve(&self, _id: NodeId, _location: Location, solver: &mut Solver) -> Result<Ty> {
        // Find our target
        let Ty::Adt(aid) = solver.resolve_name(&self.target, self.target.location())? else {
            Err(InvalidImplTarget {
                src: solver.src(self.target.location()),
                at: self.target.location().into(),
            })?
        };

        let mut pact = if let Some(name) = self.pact.as_ref() {
            let ty = solver.resolve_name(name, name.location)?;
            let Some(pid) = ty.as_single_pact() else {
                return Err(NotAPact {
                    src: solver.src(name.location),
                    at: name.location.into(),
                    ty: ty.to_string(),
                }
                .into());
            };

            // we clone because we're going to remove the fields to ensure they are all covered
            Some((solver.pacts[pid].clone(), pid))
        } else {
            None
        };

        solver.ribs.push_impl(aid);

        // todo, I am not positive that these need to be in separate passes, it may be okay to do
        // them concurrently

        // First resolve all constants
        for (con, id) in self.items.iter().filter_map(|v| {
            if let ItemKind::Const(con) = v.kind() {
                Some((con, v.id()))
            } else {
                None
            }
        }) {
            let mut ty = solver.solve_const(con, id)?;

            if let Some((pact, _)) = &mut pact {
                let mut pact_ty =
                    pact.constants
                        .swap_remove(&con.left.lexeme)
                        .ok_or_else(|| {
                            miette::Error::from(PactConstNotFound {
                                src: solver.src(con.left.location),
                                at: con.left.location.into(),
                                name: con.left.lexeme.clone(),
                                pact: pact.name.clone(),
                            })
                        })?;

                ty.fulfill_ty(&mut pact_ty, solver)
                    .map_err(|e| e.into_type_mismatch(solver, con.left.location))?;
            }
        }

        // Run again to fire off fn impls
        let result: Result<()> = self.items.iter().try_for_each(|item| {
            let ItemKind::Function(function) = item.kind() else {
                return Ok(());
            };

            // memoized solve, mirrors `Query for Expr`. top-level fns get this for free via
            // visit_stmt, but impl method items only flow through here so we have to do it
            // ourselves.
            if solver.touch_node(item.id()) {
                let ty = function.solve(item.id(), item.location(), solver)?;
                let vid = solver.node_vid(item.id());
                solver
                    .register_sub(vid, ty)
                    .map_err(|e| e.into_type_mismatch(solver, item.location()))?;
            }
            let ty = Ty::Vid(solver.node_vid(item.id())).normalized(solver);

            // inherent methods are concrete fields; refresh the stored type with the solved one.
            // pact methods aren't fields (they dispatch via `pact_impls`), so this is a no-op
            // there.
            if let Some(field) = solver.adts[aid].impls.get_mut(&function.name.lexeme) {
                field.ty = ty.clone();
            }

            if let Some((pact, _)) = &mut pact {
                let (pact_header, _default) = pact
                    .functions
                    .swap_remove(&function.name.lexeme)
                    .ok_or_else(|| {
                        miette::Error::from(PactFunctionNotFound {
                            src: solver.src(function.name.location),
                            at: function.name.location.into(),
                            name: function.name.lexeme.clone(),
                            pact: pact.name.clone(),
                        })
                    })?;

                // unify the whole function type against the (freshened) pact requirement --
                // covers param count, per-param types, and the return type in one move.
                let mut impl_ty = ty;
                let mut pact_ty = solver.instantiate_pact_fn(&pact_header);
                impl_ty
                    .fulfill_ty(&mut pact_ty, solver)
                    .map_err(|e| e.into_type_mismatch(solver, function.name.location))?;
            }

            Ok(())
        });

        result?;
        solver.ribs.pop();

        if let Some((pact, _)) = pact {
            // anything left in the cloned pact is unimplemented, except functions carrying a
            // default body, which the impl may legally omit (the dispatch table was filled at
            // hoist).
            let missing: Vec<String> = pact
                .functions
                .iter()
                .filter(|(_, (_, default))| default.is_none())
                .map(|(name, _)| name.clone())
                .chain(pact.constants.keys().cloned())
                .collect();
            if !missing.is_empty() {
                Err(PactImplIncomplete {
                    src: solver.src(self.target.location()),
                    at: self.target.location().into(),
                    target: self.target.lexeme.clone(),
                    pact: pact.name,
                    missing: missing.join(", "),
                })?;
            }
        }

        Ok(Ty::Unit)
    }
}

impl Solve for In {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        let rhs = self.right.query(solver)?;
        match rhs {
            Ty::Array(ref inner) => {
                self.left.fulfill_ty(*inner.clone(), solver)?;
            }
            Ty::Dict(_) => {
                self.left.fulfill_ty(Ty::Str, solver)?;
            }
            Ty::Str => {
                self.left.fulfill_ty(Ty::Str, solver)?;
            }
            _ => Err(InvalidInTarget {
                src: solver.src(location),
                at: location.into(),
                ty: rhs.to_string(),
            })?,
        }
        Ok(Ty::Bool)
    }
}

impl Solve for Literal {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        fn assemble(exprs: Vec<&Expr>, solver: &mut Solver) -> Result<Ty> {
            let ty = if let Some(expr) = exprs.first() {
                let mut first_ty = expr.query(solver)?;
                for expr in exprs.iter().skip(1) {
                    first_ty = if let Err(e) = expr.fulfill_ty(first_ty.clone(), solver) {
                        let found = expr.query(solver)?;
                        if let Some(ty) = Ty::coerce_option(found.clone(), first_ty.clone(), solver)
                        {
                            ty
                        } else if let Some(ty) = Ty::coerce_pacts(found, first_ty, solver) {
                            // mixed concrete types that share a pact widen to it, so
                            // `[Square, Circle]` can be a `[Draw]`
                            ty
                        } else {
                            Err(e)?
                        }
                    } else {
                        first_ty.clone()
                    };
                }
                first_ty
            } else {
                Ty::Vid(solver.vid())
            };
            Ok(ty)
        }
        Ok(match self {
            Literal::Array(exprs) => Ty::Array(Box::new(assemble(exprs.iter().collect(), solver)?)),
            Literal::Dictionary(fields) => Ty::Dict(Box::new(assemble(
                fields.iter().map(|(_, v)| v).collect(),
                solver,
            )?)),
            Literal::Struct(StructLiteral { name, fields }) => {
                let (adt, variant, layout) = match name.kind() {
                    ExprKind::Access(Access::DoubleColon { left, right }) => {
                        let lhs = left.query(solver)?;
                        let adt = match lhs {
                            Ty::Adt(adt) | Ty::Identity(adt) => adt,
                            _ => Err(NotAStruct {
                                src: solver.src(location),
                                at: location.into(),
                                ty: lhs.to_string(),
                            })?,
                        };

                        if solver.adts[adt].flags.contains(AdtFlags::IS_MODULE) {
                            let lhs = name.query(solver)?;
                            let Ty::Adt(layout) = lhs else {
                                Err(NotAStruct {
                                    src: solver.src(location),
                                    at: location.into(),
                                    ty: lhs.to_string(),
                                })?
                            };
                            let variant = solver.adts[layout]
                                .try_as_struct()
                                .ok_or_else(|| {
                                    Error::from(NotAStruct {
                                        src: solver.src(location),
                                        at: location.into(),
                                        ty: solver.adts[layout].name.clone(),
                                    })
                                })?
                                .clone();
                            (layout, variant, layout)
                        } else if !solver.adts[adt].flags.contains(AdtFlags::IS_ENUM) {
                            let name = solver.adts[adt].name.clone();
                            Err(NotAnEnum {
                                src: solver.src(location),
                                at: location.into(),
                                ty: name,
                            })?
                        } else {
                            let variant = solver.adts[adt]
                                .variants
                                .get(&right.lexeme)
                                .cloned()
                                .ok_or_else(|| {
                                    Error::from(VariantNotFound {
                                        src: solver.src(location),
                                        at: location.into(),
                                        name: right.lexeme.clone(),
                                    })
                                })?;

                            let layout = variant.layout().unwrap();
                            let Variant::Struct(struct_variant) = variant else {
                                Err(NotAStruct {
                                    src: solver.src(location),
                                    at: location.into(),
                                    ty: right.lexeme.clone(),
                                })?
                            };

                            (adt, struct_variant, layout)
                        }
                    }
                    ExprKind::Ident(ident) if ident.lexeme == "Self" => {
                        let adt = *solver.impl_target().as_ref().ok_or_else(|| {
                            Error::from(SelfOutOfContext {
                                src: solver.src(ident.location()),
                                at: ident.location().into(),
                            })
                        })?;
                        let variant = solver.adts[adt]
                            .try_as_struct()
                            .ok_or_else(|| {
                                Error::from(NotAStruct {
                                    src: solver.src(location),
                                    at: location.into(),
                                    ty: solver.adts[adt].name.clone(),
                                })
                            })?
                            .clone();
                        (adt, variant, adt)
                    }
                    ExprKind::Ident(ident) => {
                        let lhs = name.query(solver)?;
                        let dec = solver.ribs.resolve(ident).ok_or_else(|| NotFound {
                            src: solver.src(ident.location),
                            at: ident.location.into(),
                            name: ident.lexeme.clone(),
                        })?;
                        if solver.decs[dec].kind.as_adt().is_none() {
                            Err(NotAStruct {
                                src: solver.src(location),
                                at: location.into(),
                                ty: ident.lexeme.clone(),
                            })?
                        }
                        // `Ty::Identity` shows up when the named struct is the current impl
                        // target -- filter_adt rewrote it. treat it as the same adt.
                        let adt = match lhs {
                            Ty::Adt(adt) | Ty::Identity(adt) => adt,
                            _ => Err(NotAStruct {
                                src: solver.src(location),
                                at: location.into(),
                                ty: lhs.to_string(),
                            })?,
                        };
                        let variant = solver.adts[adt]
                            .try_as_struct()
                            .ok_or_else(|| {
                                Error::from(NotAStruct {
                                    src: solver.src(location),
                                    at: location.into(),
                                    ty: solver.adts[adt].name.clone(),
                                })
                            })?
                            .clone();
                        (adt, variant, adt)
                    }
                    _kind => {
                        let ty = name.query(solver)?.to_string();
                        Err(Error::from(NotAStruct {
                            src: solver.src(name.location()),
                            at: name.location().into(),
                            ty,
                        }))?
                    }
                };

                let mut prototype_fields = variant.fields.clone();
                for (field_name, value) in fields {
                    let target = prototype_fields
                        .swap_remove(&field_name.to_string())
                        .ok_or_else(|| {
                            Error::from(FieldNotFound {
                                src: solver.src(location),
                                at: location.into(),
                                field_name: field_name.to_string(),
                            })
                        })?;

                    value.fulfill_ty(target.ty, solver)?;
                }

                let remaining: Vec<String> = prototype_fields
                    .iter()
                    .filter(|(_, field)| !field.constant)
                    .map(|v| v.0.clone())
                    .collect();

                if !remaining.is_empty() {
                    Err(MissingStructFields {
                        src: solver.src(location),
                        at: location.into(),
                        fields: remaining,
                    })?;
                }

                solver.shadow_expr_ty(name.id(), Ty::Adt(layout), name.location())?;
                Ty::Adt(adt)
            }
            Literal::Tuple(members) => Ty::Tuple(
                members
                    .iter()
                    .map(|v| v.query(solver))
                    .collect::<Result<_>>()?,
            ),
            _ => unreachable!(),
        })
    }
}

impl Solve for Logical {
    fn solve(&self, _id: NodeId, _location: Location, solver: &mut Solver) -> Result<Ty> {
        self.left.fulfill_ty(Ty::Bool, solver)?;
        self.right.fulfill_ty(Ty::Bool, solver)?;
        Ok(Ty::Bool)
    }
}

impl Solve for Loop {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        let LoopRun {
            break_ty,
            collect_ty,
            ..
        } = solver.run_loop_body(&self.body)?;
        let ty = match (break_ty, collect_ty) {
            (None, _) => Ty::Never,
            (Some(Ty::Unit), None) => Ty::Unit,
            (Some(bty), None) => bty,
            (Some(Ty::Unit), Some(cty)) => array!(cty),
            (_, Some(_)) => Err(BreakValueWhileCollection {
                src: solver.src(location),
                at: location.into(),
            })?,
        };
        Ok(ty)
    }
}

impl Solve for Match {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        let scrut_ty = self.identity.query(solver)?.normalized(solver);

        let mut result: Option<Ty> = None;
        for case in self.cases.iter() {
            solver.ribs.push_block();
            solver.solve_match_pat(case.pat(), scrut_ty.clone(), false)?;
            if let Some(guard) = case.guard() {
                guard.fulfill_ty(Ty::Bool, solver)?;
            }
            let body = case.body();
            result = Some(match result.take() {
                None => body.query(solver)?,
                Some(acc) => {
                    if let Err(e) = body.fulfill_ty(acc.clone(), solver) {
                        Ty::coerce_option(body.query(solver)?, acc, solver).ok_or(e)?
                    } else {
                        acc
                    }
                }
            });
            solver.ribs.pop();
        }

        let body_ty = result.unwrap_or(Ty::Unit);
        if !self.panic_terminator
            && let Some(report) =
                crate::exhaustion::missing_witnesses(&self.cases, &scrut_ty, solver)
        {
            let summary = report.summary();
            let help = report.variant_defs.is_empty().then(|| {
                "add the missing arms, an `_` wildcard arm, or a `!` panic terminator".to_string()
            });
            Err(NonExhaustiveMatch {
                src: solver.src(location),
                at: location.into(),
                summary,
                help,
                variant_defs: report.variant_defs,
            })?
        }
        Ok(body_ty)
    }
}

impl Solve for Return {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        let mut this_ty = self
            .value
            .as_ref()
            .map_or(Ok(Ty::Unit), |e| e.query(solver))?;

        let mut expected_ty = solver
            .fn_stack
            .last()
            .ok_or_else(|| ReturnOutOfFunction {
                src: solver.src(location),
                at: location.into(),
            })?
            .expected_ty
            .clone();

        this_ty
            .fulfill_ty(&mut expected_ty, solver)
            .map_err(|e| e.into_type_mismatch(solver, location))?;

        solver.control_flow.push(Flow::Return);

        Ok(Ty::Never)
    }
}

impl Solve for Struct {
    fn solve(&self, _id: NodeId, _location: Location, solver: &mut Solver) -> Result<Ty> {
        let Ty::Adt(adt) = self.name.query(solver)? else {
            unreachable!()
        };

        // push an impl scope so field types that reference this struct (e.g. `struct Foo { f: Foo
        // }`) get filter_adt-ed to `Ty::Identity` -- breaks the recursion check that would
        // otherwise refuse the self-referential adt.
        solver.ribs.push_impl(adt);

        let is_tuple = solver.adts[adt].is_tuple_struct();

        let result = self.fields.iter().enumerate().try_for_each(|(i, field)| {
            let mut member_ty = if is_tuple {
                solver.adts[adt].as_tuple().members[i].clone()
            } else {
                solver.adts[adt]
                    .as_struct_mut()
                    .fields
                    .get_mut(&field.name.to_string())
                    .unwrap()
                    .ty
                    .clone()
            };

            let mut ty = Ty::from_annotation(field.annotation.clone(), solver)?;
            member_ty
                .fulfill_ty(&mut ty, solver)
                .map_err(|v| v.into_type_mismatch(solver, field.location))
        });

        solver.ribs.pop();
        result?;

        let mut normalized = solver.adts[adt].clone();
        normalized.normalize_members(solver);
        solver.adts[adt] = normalized;

        Ok(Ty::Adt(adt))
    }
}

impl Solve for Unary {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        fn unary(op: UnaryOp, ty: &Ty, location: Location, solver: &Solver) -> Result<Ty> {
            match (op, ty) {
                (UnaryOp::Not, Ty::Bool) => Ok(Ty::Bool),
                (UnaryOp::BitwiseNot, Ty::Int) => Ok(Ty::Int),
                (UnaryOp::Negative | UnaryOp::Positive, Ty::Int) => Ok(Ty::Int),
                (UnaryOp::Negative | UnaryOp::Positive, Ty::Float) => Ok(Ty::Float),
                (op, Ty::Tuple(elems)) => elems
                    .iter()
                    .map(|e| unary(op, e, location, solver))
                    .collect::<Result<Vec<Ty>>>()
                    .map(Ty::Tuple),
                _ => Err(InvalidUnary {
                    src: solver.src(location),
                    at: location.into(),
                    ty: ty.to_string(),
                    op: op.to_string(),
                })?,
            }
        }

        // for an unresolved operand, pin it to the scalar shape this op needs before checking
        let ty = self.right.query(solver)?.normalized(solver);
        if matches!(ty, Ty::Vid(_)) {
            let pin = match self.op {
                UnaryOp::Not => Ty::Bool,
                UnaryOp::BitwiseNot => Ty::Int,
                UnaryOp::Negative | UnaryOp::Positive => return Ok(ty),
            };
            self.right.fulfill_ty(pin.clone(), solver)?;
            return Ok(pin);
        }
        unary(self.op, &ty, location, solver)
    }
}

impl Solve for Unwrap {
    fn solve(&self, _id: NodeId, _location: Location, solver: &mut Solver) -> Result<Ty> {
        let ty = self.expr.query(solver)?;
        let new_ty = match ty.normalized(solver) {
            Ty::Option(inner) | Ty::Result(inner) => *inner,
            ty => Err(Error::from(InvalidUnwrap {
                src: solver.src(self.expr.location()),
                at: self.expr.location().into(),
                ty: ty.to_string(),
            }))?,
        };
        Ok(new_ty)
    }
}

impl Solve for parse::Raise {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        let expected_ty = solver
            .fn_stack
            .last()
            .ok_or_else(|| RaiseOutsideResult {
                src: solver.src(location),
                at: location.into(),
            })?
            .expected_ty
            .clone();

        if !matches!(expected_ty.normalized(solver), Ty::Result(_)) {
            Err(RaiseOutsideResult {
                src: solver.src(location),
                at: location.into(),
            })?
        }

        self.value.fulfill_ty(Ty::Str, solver)?;
        solver.control_flow.push(Flow::Return);
        Ok(Ty::Never)
    }
}

impl Solve for parse::Range {
    fn solve(&self, _: NodeId, _: Location, solver: &mut Solver) -> Result<Ty> {
        self.start.fulfill_ty(Ty::Int, solver)?;
        self.end.fulfill_ty(Ty::Int, solver)?;
        Ok(Ty::Int)
    }
}

impl Solve for parse::Absolve {
    fn solve(&self, _id: NodeId, _location: Location, solver: &mut Solver) -> Result<Ty> {
        let lhs = self.left.query(solver)?.normalized(solver);
        let Ty::Result(inner) = lhs else {
            Err(AbsolveNonResult {
                src: solver.src(self.left.location()),
                at: self.left.location().into(),
                ty: lhs.to_string(),
            })?
        };
        let inner = *inner;
        let handler_ty = Ty::Fn(FnHeader::new(
            vec![FnParam::new(None, Ty::Str, false)],
            inner.clone(),
            false,
        ));
        self.handler.fulfill_ty(handler_ty, solver)?;
        Ok(inner)
    }
}

impl Solve for While {
    fn solve(&self, _id: NodeId, location: Location, solver: &mut Solver) -> Result<Ty> {
        let mut pushed_rib = false;
        if let Some(binding) = self.binding.as_ref() {
            solver.ribs.push_block();
            pushed_rib = true;
            let header_ty = self.header.query(solver)?;
            let binding_ty = if let Ty::Option(ty) = header_ty {
                *ty
            } else {
                header_ty
            };
            solver.solve_pat(binding, binding_ty)?;
        } else {
            self.header.fulfill_ty(Ty::Bool, solver)?;
        };

        let LoopRun {
            break_ty,
            collect_ty,
            ..
        } = solver.run_loop_body(&self.body)?;

        if pushed_rib {
            solver.ribs.pop();
        }

        // Validate the types
        let break_ty = break_ty.unwrap_or_default();
        let ty = match (break_ty, collect_ty) {
            (Ty::Unit, None) => Ty::Unit,
            (bty, None) => Ty::Option(Box::new(bty)),
            (Ty::Unit, Some(cty)) => array!(cty),
            (_, Some(_)) => Err(BreakValueWhileCollection {
                src: solver.src(location),
                at: location.into(),
            })?,
        };
        Ok(ty)
    }
}

/// Pulls an `AdtId` out of an expression that names a type (the left of a `::`). A bare tuple
/// struct ident resolves to its constructor `Ty::Fn` via `Ident::solve`, so look through that to
/// reach the underlying adt.
fn adt_from_type_path(left: &Expr, solver: &mut Solver) -> Result<AdtId> {
    let ty = left.query(solver)?;
    match ty {
        Ty::Adt(adt) | Ty::Identity(adt) => Ok(adt),
        Ty::Fn(ref f) => match f.return_ty.as_ref() {
            Ty::Adt(adt) | Ty::Identity(adt) => Ok(*adt),
            _ => Err(TypeHasNoFields {
                src: solver.src(left.location()),
                at: left.location().into(),
                ty: ty.to_string(),
            })?,
        },
        _ => Err(TypeHasNoFields {
            src: solver.src(left.location()),
            at: left.location().into(),
            ty: ty.to_string(),
        })?,
    }
}
