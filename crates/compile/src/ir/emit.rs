use crate::{
    OperandKind,
    ir::{BinOp, BlockId, BodyId, Constant, FormatPart, Inst, InstId, Ir, LoopCtx, Place, UnaryOp},
};
use api::NativeId;
use parse::{
    Absolve, Access, AccessKind, AssignmentOp, Block, Break, Call, Closure, Coalescence, Collect,
    Continue, Equality, EqualityOp, Evaluation, Expr, ExprKind, FString, FStringPart, For,
    Grouping, Ident, If, In, Literal, Logical, Loop, Match, MatchCase, NodeId, Raise, Range,
    Return, Stmt, StmtKind, Unary, Unwrap, While,
    components::{Binding, Pat, PatKind},
};
use shared::{AdtId, Located, PactId};
use solve::{
    ResolvedAdt, ResolvedDeclKind,
    components::{ConstValue, DecId, FnHeader, Ty},
};

// per-kind worker -- mirrors `Solve`. source location is ambient on `Ir.current_loc`,
// set by `Lower for Expr` / `Ir::stmt` before dispatch.
pub(crate) trait Emit {
    #[must_use]
    fn emit(&self, id: NodeId, ir: &mut Ir) -> Option<InstId>;
}

// Expr-level entry -- mirrors `Query`; owns the id and feeds it to the kind
pub(crate) trait Lower {
    #[must_use]
    fn lower(&self, ir: &mut Ir) -> Option<InstId>;
}

impl Ir {
    pub(crate) fn stmt(&mut self, stmt: &Stmt) -> Option<()> {
        self.with_loc(stmt.location(), |ir| match stmt.kind() {
            StmtKind::Let(s) => {
                let value = s.right.lower(ir)?;
                match s.else_branch.as_ref() {
                    None => ir.current().test_pattern(&s.left, value, None),
                    Some(else_expr) => {
                        let otherwise = ir.push_block("let_else");
                        let cont = ir.push_block("let_cont");
                        ir.current().test_pattern(&s.left, value, Some(otherwise));
                        ir.current().jump(cont);
                        ir.target(otherwise);
                        let _ = else_expr.lower(ir);
                        // else must diverge; cap the block if it didn't terminate itself (e.g. a
                        // `panic` call emits no terminator of its own).
                        if !ir.is_terminated() {
                            let null = ir.current().constant(Constant::Null);
                            ir.current().ret(null);
                        }
                        ir.target(cont);
                    }
                }
                Some(())
            }
            StmtKind::Module(_) => Some(()),
            StmtKind::Item(item) => {
                match item.kind() {
                    parse::ItemKind::Poison(poison) => poison.escaped(),
                    // function bodies need to be lowered into their own Body
                    parse::ItemKind::Function(f) => {
                        let _ = f.emit(item.id(), ir);
                    }
                    // impl walks its inner items
                    parse::ItemKind::Impl(i) => {
                        let _ = i.emit(item.id(), ir);
                    }
                    // defaulted method bodies lower once into the shared default dec's
                    // body -- every impl that omits the method grafts a field pointing at
                    // that dec (see solve's hoist for Impl)
                    parse::ItemKind::Pact(p) => {
                        for item in &p.items {
                            if let parse::PactItem::Fn {
                                parameters,
                                default: Some(body),
                                ..
                            } = item
                            {
                                let dec = ir.node_dec(body.id());
                                let bid = ir.item_body_for(dec);
                                ir.in_body(bid, |ir| ir.lower_fn_body(parameters, body));
                            }
                        }
                    }
                    // these have no runtime presence
                    parse::ItemKind::Struct(_)
                    | parse::ItemKind::Enum(_)
                    | parse::ItemKind::Const(_)
                    | parse::ItemKind::Use(_) => {}
                }
                Some(())
            }
            StmtKind::Assignment(ass) => {
                let place = ass.left.place(ir)?;
                let value = match ass.op {
                    AssignmentOp::Identity => ass.right.lower(ir)?,
                    AssignmentOp::NullCoalescenceEqual => {
                        let lhs = place.load(ir);
                        emit_coalesce(ir, lhs, &ass.right)?
                    }
                    op => {
                        // the compound op's result type is the place's type (a typed lvalue); a
                        // mismatched runtime operand falls back to the generic op.
                        let kind = num_kind(ir, stmt.id(), &ass.left, &ass.right);
                        let lhs = place.load(ir);
                        let rhs = ass.right.lower(ir)?;
                        ir.current().bin(op.into(), lhs, rhs, kind)
                    }
                };
                let is_scalar = matches!(
                    ir.resolutions.node_tys.get(&ass.left.id()),
                    Some(Ty::Int | Ty::Float | Ty::Bool)
                );
                place.store(ir, value, is_scalar);

                Some(())
            }
            StmtKind::Expr(expr) => {
                // the result of this emit is tossed out beyond whatever knock on effects it did --
                // nothing can consume it
                let _ = expr.lower(ir);

                // what we actually care about is our _state_, regardless of what the expr yielded.
                // if we're terminated, we can return None here. this lets us not inject synthetic
                // nulls in functions
                if ir.is_terminated() { None } else { Some(()) }
            }
        })
    }

    /// Binds parameters as locals of the current body and lowers `body`, emitting a trailing
    /// null-return if control flow reached the end without an explicit terminator. Used by both
    /// function and closure emit; the caller picks the body id and any captures.
    pub(crate) fn lower_fn_body(&mut self, parameters: &[Binding], body: &Expr) {
        let params: Vec<_> = parameters
            .iter()
            .map(|b| {
                let dec = self.node_dec(b.left.id());
                self.local_for(dec)
            })
            .collect();
        self.current_body_mut().params = params;

        let body_value = body.lower(self);
        self.finish(body_value);
    }
}

impl Emit for Access {
    fn emit(&self, id: NodeId, ir: &mut Ir) -> Option<InstId> {
        match self {
            Access::Identity { right: _ } => todo!(),
            Access::Dot { left, right, kind } => {
                // a pact-typed receiver has no fields, so this reads a pact constant, and the
                // receiver's own impl of it is picked at runtime
                let optional = *kind == AccessKind::Option || left.taints_chain();
                let pact_pids = match ir.resolutions.node_tys.get(&left.id()) {
                    Some(Ty::Option(inner)) if optional => inner.as_pacts(),
                    Some(ty) => ty.as_pacts(),
                    None => None,
                };
                if let Some(pids) = pact_pids {
                    let name = &right.as_ident().expect("constant name is an ident").lexeme;
                    let recv = left.lower(ir)?;
                    let candidates = pact_candidates(ir, &pids, |adt| adt.constants[name]);
                    let read = |ir: &mut Ir| {
                        pact_dispatch(ir, recv, candidates, |ir, dec| {
                            Constant::emit_const_dec(ir, dec)
                        })
                    };
                    return if optional {
                        unless_null(ir, recv, read)
                    } else {
                        read(ir)
                    };
                }

                // associated-const access: solver resolved this to a Constant dec.
                // emit the receiver for side effects, then load the folded value.
                if let Some(dec) = ir.try_node_dec(id)
                    && let ResolvedDeclKind::Constant(_) = &ir.resolutions.decs[dec].kind
                {
                    let _ = left.lower(ir)?;
                    return Constant::emit_const_dec(ir, dec);
                }

                // struct fields and tuple indices are positional and known at IR time, so
                // we go directly to a slotted GetField instead of synthesizing a const + GetIndex.
                let kind = if *kind == AccessKind::Option || left.taints_chain() {
                    AccessKind::Option
                } else {
                    AccessKind::Direct
                };
                let recv = left.id();
                let is_struct = kind == AccessKind::Direct
                    && matches!(ir.resolutions.node_tys.get(&recv), Some(Ty::Adt(_)));
                let left = lower_shared(ir, left)?;
                let slot = match right.kind() {
                    ExprKind::Literal(Literal::Int(i)) => u32::try_from(*i).unwrap(),
                    ExprKind::Ident(ident) => {
                        u32::try_from(ir.field_index(recv, &ident.lexeme)).unwrap()
                    }
                    _ => unreachable!(),
                };
                let value = ir.current().get_field(left, slot, kind, is_struct);
                let plain = ir.resolutions.node_tys.get(&id).is_some_and(Ty::is_plain);
                Some(owned(ir, value, plain))
            }
            Access::DoubleColon { .. } => {
                let dec = ir.node_dec(id);
                match &ir.resolutions.decs[dec].kind {
                    ResolvedDeclKind::Variant { layout, .. } => {
                        let layout = *layout;
                        Some(ir.current().new_instance(layout, vec![]))
                    }
                    ResolvedDeclKind::Item { .. } => {
                        let body = ir.item_body_for(dec);
                        Some(ir.current().ref_body(body))
                    }
                    ResolvedDeclKind::Constant(_) => Constant::emit_const_dec(ir, dec),
                    ResolvedDeclKind::Adt(adt) => {
                        let adt = *adt;
                        Some(adt_value(ir, dec, adt))
                    }
                    ResolvedDeclKind::Local => unreachable!("`::` never resolves to a local"),
                    ResolvedDeclKind::Pact(_) => todo!(),
                }
            }
            // only a constant gets here (a fn is lowered by its call)
            Access::Each { left, right } => {
                let array = ir.current().new_array();
                for dec in ir.each_impl(left.id(), &right.lexeme) {
                    let value = Constant::emit_const_dec(ir, dec)?;
                    ir.current().push(array, value);
                }
                Some(array)
            }
            Access::Square { left, key, kind } => {
                // tainted plain accesses ride an upstream `?`, so they must null-check at runtime
                // just like an explicit `?[i]` -- else a mid-chain null faults instead of flowing
                // through. same effective-kind rule the solver types against.
                let kind = if *kind == AccessKind::Option || left.taints_chain() {
                    AccessKind::Option
                } else {
                    AccessKind::Direct
                };
                let is_array = kind == AccessKind::Direct
                    && matches!(ir.resolutions.node_tys.get(&left.id()), Some(Ty::Array(_)));
                let left = lower_shared(ir, left)?;
                let key = key.lower(ir)?;
                let value = ir.current().get_index(left, key, kind, is_array);
                let plain = ir.resolutions.node_tys.get(&id).is_some_and(Ty::is_plain);
                Some(owned(ir, value, plain))
            }
        }
    }
}

impl Emit for Block {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        // plumbing (entry/continuation jumps, implicit-null yield) inherits the
        // enclosing Expr's Real loc set by `Lower for Expr` -- good enough for any
        // runtime fault attribution here, and Lower::lower / Ir::stmt restore precise
        // locs during recursion.
        let block_id = ir.push_block("block");
        ir.current().jump(block_id);
        ir.target(block_id);

        for stmt in &self.body {
            ir.stmt(stmt)?;
        }

        let value = match &self.yielded_expr {
            Some(expr) => expr.lower(ir)?,
            None => ir.current().constant(Constant::Null),
        };

        let continuation = ir.push_block("continuation");
        ir.current().jump(continuation);
        ir.target(continuation);

        Some(value)
    }
}

impl Emit for Break {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let value = match self.value.as_ref() {
            Some(value) => value.lower(ir)?,
            None => ir.current().constant(Constant::Null),
        };

        let block = ir.current_block_id();
        let this_loop = ir
            .loop_stack_mut()
            .last_mut()
            .expect("attempted to emit break outside of a loop");
        let exit = this_loop.exit;
        this_loop.breaks.push((block, value));

        ir.current().jump(exit);

        None
    }
}

impl Emit for Call {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        fn fill_call_args(
            ir: &mut Ir,
            fn_ty: &FnHeader,
            defaults: &[Option<parse::Literal>],
            arguments: &[parse::Argument],
            receiver: Option<InstId>,
        ) -> Option<Vec<InstId>> {
            let extra = (receiver.is_some() && !fn_ty.is_method) as usize;
            let total = fn_ty.parameters.len() + extra;
            let mut slots: Vec<Option<InstId>> = vec![None; total];
            let mut cursor = 0;
            if let Some(recv) = receiver {
                slots[0] = Some(recv);
                cursor = 1;
            }
            for arg in arguments {
                let slot = match &arg.name {
                    Some(name) => fn_ty
                        .parameters
                        .iter()
                        .position(|p| p.name.as_deref() == Some(name.lexeme.as_str()))
                        .map(|i| i + extra)
                        .expect("solver validated this named arg maps to a slot"),
                    None => {
                        let s = cursor;
                        cursor += 1;
                        s
                    }
                };
                slots[slot] = Some(arg.value.lower(ir)?);
            }
            Some(
                (0..total)
                    .map(|i| match slots[i] {
                        Some(inst) => inst,
                        None => {
                            // no literal default means this is a native's omitted trailing
                            // Option param (fog fn defaults always carry a literal) -- fill
                            // with null, which marshals to None
                            let lit = defaults
                                .get(i)
                                .cloned()
                                .flatten()
                                .unwrap_or(parse::Literal::Null);
                            let constant = Constant::from_literal(ir, lit);
                            ir.current().constant(constant)
                        }
                    })
                    .collect(),
            )
        }

        fn dec_defaults(ir: &Ir, dec: Option<DecId>) -> Vec<Option<parse::Literal>> {
            dec.and_then(|d| match &ir.resolutions.decs[d].kind {
                ResolvedDeclKind::Item { defaults, .. } => Some(defaults.clone()),
                _ => None,
            })
            .unwrap_or_default()
        }

        fn emit_intrinsic(
            ir: &mut Ir,
            intrinsic: api::Intrinsic,
            args: &[InstId],
            call: &Call,
        ) -> InstId {
            match intrinsic {
                api::Intrinsic::Len => ir.current().len(args[0]),
                api::Intrinsic::In => ir.current().check_in(args[1], args[0], true),
                api::Intrinsic::Push => {
                    ir.current().push(args[0], args[1]);
                    ir.current().constant(Constant::Null)
                }
                api::Intrinsic::ToFloat => ir.current().to_float(args[0]),
                api::Intrinsic::Sqrt => ir.current().sqrt(args[0]),
                api::Intrinsic::File => {
                    let name = ir
                        .resolutions
                        .sources
                        .get(&call.left.location().file_id)
                        .unwrap()
                        .name()
                        .to_string();
                    let path = std::fs::canonicalize(&name)
                        .map_or(name, |path| path.to_string_lossy().into_owned());
                    let path = ir.intern_str(&path);
                    ir.current().constant(Constant::Str(path))
                }
            }
        }

        // pact-typed receiver dispatch: a runtime IsInstance chain over implementors of the bound,
        // each arm CallDirect-ing that adt's body for the method. a lone implementor is tested too,
        // since a session can add another after this call site is compiled
        fn emit_pact_dispatch(
            ir: &mut Ir,
            fn_ty: &FnHeader,
            recv: InstId,
            arguments: &[parse::Argument],
            pids: &[PactId],
            method: &str,
        ) -> Option<InstId> {
            // lower args once -- fill_call_args evaluates them, so per-arm calls re-run side
            // effects.
            let args = fill_call_args(ir, fn_ty, &[], arguments, fn_ty.is_method.then_some(recv))?;

            let candidates = pact_candidates(ir, pids, |adt| adt.methods[method]);
            let candidates: Vec<_> = candidates
                .into_iter()
                .map(|(dec, ids)| (ir.item_body_for(dec), ids))
                .collect();
            pact_dispatch(ir, recv, candidates, |ir, body| {
                Some(ir.current().call_direct(body, args.clone()))
            })
        }

        let callee_ty = ir.resolutions.node_tys.get(&self.left.id());
        if let Some(Ty::Adt(adt)) = callee_ty {
            let adt = *adt;
            let mut args = Vec::with_capacity(self.arguments.len());
            for arg in &self.arguments {
                debug_assert!(arg.name.is_none(), "solver rejects named args on adt ctor");
                args.push(arg.value.lower(ir)?);
            }
            return Some(ir.current().new_instance(adt, args));
        }
        // tuple-struct ctor used as a value (`let make = Wrap; make(1)`): the callee's `Ty::Fn`
        // carries `is_ctor` from `Ident::solve`. lower to `NewInstance` directly -- the body
        // it'd otherwise dispatch through doesn't exist.
        if let Some(Ty::Fn(header)) = callee_ty
            && header.is_ctor
            && let Ty::Adt(adt) = header.return_ty.as_ref()
        {
            let adt = *adt;
            let mut args = Vec::with_capacity(self.arguments.len());
            for arg in &self.arguments {
                debug_assert!(
                    arg.name.is_none(),
                    "solver rejects named args on tuple-ctor Fn value"
                );
                args.push(arg.value.lower(ir)?);
            }
            return Some(ir.current().new_instance(adt, args));
        }

        let fn_ty = match callee_ty {
            Some(Ty::Fn(h)) => h.clone(),
            _ => unreachable!("solver typed callee as something other than Adt or Fn"),
        };

        // one call per implementer known here (a session can add another after this is compiled),
        // all sharing the same lowered args
        if let ExprKind::Access(Access::Each { left, right }) = self.left.kind() {
            let args = fill_call_args(ir, &fn_ty, &[], &self.arguments, None)?;
            let array = ir.current().new_array();
            for dec in ir.each_impl(left.id(), &right.lexeme) {
                let body = ir.item_body_for(dec);
                let value = ir.current().call_direct(body, args.clone());
                ir.current().push(array, value);
            }
            return Some(array);
        }

        if let ExprKind::Access(Access::Dot {
            left,
            right,
            kind: access_kind,
        }) = self.left.kind()
        {
            // pact-typed receiver (`g: Greet`): no single concrete callee, so dispatch at runtime
            // over every implementor of the bound. a grafted default lands in each impl's table,
            // so default methods dispatch here for free.
            // `self` inside a default body counts too: one implementer, decided at runtime
            // through `?.` the receiver is an option of the bound, and null skips the dispatch
            let optional = *access_kind == AccessKind::Option || left.taints_chain();
            let pact_pids = match ir.resolutions.node_tys.get(&left.id()) {
                Some(Ty::Option(inner)) if optional => inner.as_pacts(),
                Some(ty) => ty.as_pacts(),
                None => None,
            };
            if let Some(pids) = pact_pids {
                let method = right
                    .as_ident()
                    .expect("method name is an ident")
                    .lexeme
                    .clone();
                let recv = left.lower(ir)?;
                let dispatch = |ir: &mut Ir| {
                    emit_pact_dispatch(ir, &fn_ty, recv, &self.arguments, &pids, &method)
                };
                return if optional {
                    unless_null(ir, recv, dispatch)
                } else {
                    dispatch(ir)
                };
            }

            let access_id = self.left.id();
            if let Some(&dec) = ir.resolutions.node_decs.get(&access_id)
                && let ResolvedDeclKind::Item {
                    native, takes_self, ..
                } = ir.resolutions.decs[dec].kind
            {
                let native_id = native;
                let body = (native_id.is_none()).then(|| ir.item_body_for(dec));
                let defaults = dec_defaults(ir, Some(dec));
                // `len` and `contains` only read their receiver
                let reads = native_id.is_some_and(|id| {
                    matches!(
                        ir.intrinsics.get(&id),
                        Some(api::Intrinsic::Len | api::Intrinsic::In)
                    )
                });
                let receiver = if reads {
                    lower_shared(ir, left)
                } else {
                    left.lower(ir)
                }?;

                let do_call = |ir: &mut Ir, receiver: InstId| -> Option<InstId> {
                    let receiver = takes_self.then_some(receiver);
                    let args = fill_call_args(ir, &fn_ty, &defaults, &self.arguments, receiver)?;
                    Some(match (native_id, body) {
                        (Some(id), _) => match ir.intrinsics.get(&id) {
                            Some(&i) => emit_intrinsic(ir, i, &args, self),
                            None => ir.current().call_native(id, args),
                        },
                        (None, Some(body)) => ir.current().call_direct(body, args),
                        _ => unreachable!(),
                    })
                };

                // for `obj?.method(...)`: emit a runtime null-check on the receiver. if null,
                // the call's result is null without invoking the method (matches the solver's
                // Option-wrapped return type). if non-null, do the call. otherwise lower as-is.
                // a tainted receiver (an upstream `?` in the chain) null-checks the same way.
                if *access_kind == AccessKind::Option || left.taints_chain() {
                    return unless_null(ir, receiver, |ir| do_call(ir, receiver));
                }

                return do_call(ir, receiver);
            }
        }

        enum StaticCallee {
            Body(BodyId),
            Native(NativeId),
        }
        let (static_callee, callee_dec) = match self.left.kind() {
            ExprKind::Ident(_) | ExprKind::Access(Access::DoubleColon { .. }) => {
                let dec = ir.node_dec(self.left.id());
                match &ir.resolutions.decs[dec].kind {
                    ResolvedDeclKind::Item {
                        native: Some(id), ..
                    } => (Some(StaticCallee::Native(*id)), Some(dec)),
                    ResolvedDeclKind::Item { native: None, .. } => {
                        (Some(StaticCallee::Body(ir.item_body_for(dec))), Some(dec))
                    }
                    _ => (None, None),
                }
            }
            _ => (None, None),
        };

        let defaults = dec_defaults(ir, callee_dec);
        let args = fill_call_args(ir, &fn_ty, &defaults, &self.arguments, None)?;

        Some(match static_callee {
            Some(StaticCallee::Body(body)) => ir.current().call_direct(body, args),
            Some(StaticCallee::Native(id)) => match ir.intrinsics.get(&id) {
                Some(&i) => emit_intrinsic(ir, i, &args, self),
                None => ir.current().call_native(id, args),
            },
            None => {
                let callee = self.left.lower(ir)?;
                ir.current().call(callee, args)
            }
        })
    }
}

impl Emit for Closure {
    fn emit(&self, id: NodeId, ir: &mut Ir) -> Option<InstId> {
        // each closure expr emits exactly once, so take ownership rather than cloning
        let captured_decs: Vec<DecId> = ir
            .resolutions
            .closure_captures
            .swap_remove(&id)
            .unwrap_or_default();

        // read captures from the outer body's locals first -- `in_body` swaps context away from it
        let capture_insts: Vec<InstId> = captured_decs
            .iter()
            .map(|&dec| {
                let local = ir.local_for(dec);
                ir.current().get_local(local)
            })
            .collect();

        let bid = ir.bodies.push(crate::ir::Body::new());
        ir.closure_bodies.insert(bid, id);

        ir.in_body(bid, |ir| {
            let capture_locals: Vec<_> = captured_decs.iter().map(|&d| ir.local_for(d)).collect();
            ir.current_body_mut().captures = capture_locals;
            ir.lower_fn_body(&self.parameters, &self.body);
        });

        Some(ir.current().make_closure(bid, capture_insts))
    }
}

impl Emit for Coalescence {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let lhs = self.left.lower(ir)?;
        emit_coalesce(ir, lhs, &self.right)
    }
}

impl Emit for Collect {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let value = self.value.lower(ir)?;
        let block = ir.current_block_id();
        let this_loop = ir
            .loop_stack_mut()
            .last_mut()
            .expect("attempted to emit collect outside of a loop");
        let continuation = this_loop.continuation;
        let collection = this_loop
            .collection
            .expect("attempted to emit collect in a non-collecting loop");
        this_loop.collects.push(block);

        ir.in_current(|block| {
            block.push(collection, value);
            block.jump(continuation);
        });

        None
    }
}

impl Emit for Continue {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let block = ir.current_block_id();
        let this_loop = ir
            .loop_stack_mut()
            .last_mut()
            .expect("attempted to emit continue outside of a loop");
        let continuation = this_loop.continuation;
        this_loop.continues.push(block);

        ir.current().jump(continuation);

        None
    }
}

impl Emit for Equality {
    fn emit(&self, id: NodeId, ir: &mut Ir) -> Option<InstId> {
        fn fieldless_layout(ir: &Ir, expr: &Expr) -> Option<AdtId> {
            match expr.kind() {
                ExprKind::Grouping(grouping) => fieldless_layout(ir, &grouping.inner),
                ExprKind::Ident(_) | ExprKind::Access(Access::DoubleColon { .. }) => {
                    match &ir.resolutions.decs[ir.node_dec(expr.id())].kind {
                        ResolvedDeclKind::Variant { layout: adt, .. }
                        | ResolvedDeclKind::Adt(adt) => {
                            ir.resolutions.adts[*adt].fields.is_empty().then_some(*adt)
                        }
                        ResolvedDeclKind::Constant(ConstValue::Expr(value)) => {
                            fieldless_layout(ir, value)
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        }

        // comparing to a value with no fields only needs the other side's layout
        if matches!(self.op, EqualityOp::Equal | EqualityOp::NotEqual) {
            let fieldless = fieldless_layout(ir, &self.right)
                .map(|layout| (&self.left, layout))
                .or_else(|| fieldless_layout(ir, &self.left).map(|layout| (&self.right, layout)));
            if let Some((other, layout)) = fieldless {
                let other = lower_shared(ir, other)?;
                let same = ir.current().is_instance(other, layout);
                return Some(match self.op {
                    EqualityOp::Equal => same,
                    _ => ir.current().unary(UnaryOp::Not, same),
                });
            }

            let kind = num_kind(ir, id, &self.left, &self.right);
            let left = lower_shared(ir, &self.left)?;
            let right = lower_shared(ir, &self.right)?;
            // a shared constant still compares as two independent values (including any NaNs)
            if kind == OperandKind::Generic && (is_shared(ir, left) || is_shared(ir, right)) {
                let same = ir.current().bin(BinOp::StructuralEqual, left, right, kind);
                return Some(match self.op {
                    EqualityOp::Equal => same,
                    _ => ir.current().unary(UnaryOp::Not, same),
                });
            }
            return Some(ir.current().bin(self.op.into(), left, right, kind));
        }

        emit_bin_op(ir, id, &self.left, self.op, &self.right)
    }
}

impl Emit for Evaluation {
    fn emit(&self, id: NodeId, ir: &mut Ir) -> Option<InstId> {
        emit_bin_op(ir, id, &self.left, self.op, &self.right)
    }
}

impl Lower for Expr {
    fn lower(&self, ir: &mut Ir) -> Option<InstId> {
        let id = self.id();
        ir.with_loc(self.location(), |ir| match self.kind() {
            ExprKind::Poison(poison) => poison.escaped(),
            ExprKind::Absolve(absolve) => absolve.emit(id, ir),
            ExprKind::Access(access) => access.emit(id, ir),
            ExprKind::Block(block) => block.emit(id, ir),
            ExprKind::Break(b) => b.emit(id, ir),
            ExprKind::Call(call) => call.emit(id, ir),
            ExprKind::Closure(closure) => closure.emit(id, ir),
            ExprKind::Coalescence(coalescence) => coalescence.emit(id, ir),
            ExprKind::Collect(collect) => collect.emit(id, ir),
            ExprKind::Continue(c) => c.emit(id, ir),
            ExprKind::Equality(equality) => equality.emit(id, ir),
            ExprKind::Evaluation(evaluation) => evaluation.emit(id, ir),
            ExprKind::For(for_expr) => for_expr.emit(id, ir),
            ExprKind::FString(f) => f.emit(id, ir),
            ExprKind::Grouping(g) => g.emit(id, ir),
            ExprKind::Ident(i) => i.emit(id, ir),
            ExprKind::If(if_expr) => if_expr.emit(id, ir),
            ExprKind::In(i) => i.emit(id, ir),
            ExprKind::Literal(lit) => lit.emit(id, ir),
            ExprKind::Logical(logical) => logical.emit(id, ir),
            ExprKind::Loop(loop_expr) => loop_expr.emit(id, ir),
            ExprKind::Match(m) => m.emit(id, ir),
            ExprKind::Raise(raise) => raise.emit(id, ir),
            ExprKind::Range(_) => {
                unreachable!("ranges aren't currently supported as independent exprs")
            }
            ExprKind::Return(ret) => ret.emit(id, ir),
            ExprKind::Unary(unary) => unary.emit(id, ir),
            ExprKind::Unwrap(un) => un.emit(id, ir),
            ExprKind::While(while_expr) => while_expr.emit(id, ir),
        })
    }
}

impl Emit for For {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let collection = loop_collects(&self.body).then(|| ir.current().new_array());

        let (idx_local, start, frozen_bound, seq) = match self.iterator.kind() {
            ExprKind::Range(Range {
                start,
                end,
                inclusive,
            }) => {
                let var = match ir.try_node_dec(self.binding.id()) {
                    Some(d) => ir.local_for(d),
                    None => ir.synthetic_local("$fidx"),
                };
                let start_v = start.lower(ir)?;
                let end_v = end.lower(ir)?;
                let bound = if *inclusive {
                    let one = ir.current().constant(1);
                    ir.current().bin(BinOp::Add, end_v, one, OperandKind::Int)
                } else {
                    end_v
                };
                (var, start_v, Some(bound), None)
            }
            _ => {
                let int_iter = match self.iterator.kind() {
                    ExprKind::Literal(Literal::Int(_)) => true,
                    ExprKind::Literal(Literal::String(_)) => false,
                    _ => ir.resolutions.node_tys[&self.iterator.id()] == Ty::Int,
                };
                let target = lower_shared(ir, &self.iterator)?;
                let zero = ir.current().constant(0);
                if int_iter {
                    let var = match ir.try_node_dec(self.binding.id()) {
                        Some(d) => ir.local_for(d),
                        None => ir.synthetic_local("$fidx"),
                    };
                    (var, zero, Some(target), None)
                } else {
                    let dict_iter = matches!(
                        ir.resolutions.node_tys.get(&self.iterator.id()),
                        Some(Ty::Dict(_))
                    );
                    let frozen = (!body_may_mutate_len(&self.body, dict_iter))
                        .then(|| ir.current().len(target));
                    (ir.synthetic_local("$fidx"), zero, frozen, Some(target))
                }
            }
        };

        let header = ir.push_block("for_header");
        let latch = ir.push_block("for_latch");
        let empty = ir.push_block("for_empty");
        let exit = ir.push_block("for_exit");

        let null = ir.current().constant(Constant::Null);
        ir.current().set_local(idx_local, start);
        let bound = frozen_bound.unwrap_or_else(|| ir.current().len(seq.unwrap()));
        let entered = ir
            .current()
            .bin(BinOp::LessThan, start, bound, OperandKind::Int);
        ir.current().jump_if_false(entered, empty);
        ir.current().jump(header);

        ir.target(empty);
        ir.current().jump(exit);

        ir.target(header);
        // A bare name on an integer/range loop already is the index local.
        if seq.is_some() || !matches!(self.binding.kind(), PatKind::Ident(_)) {
            let i = ir.current().get_local(idx_local);
            let elem = if let Some(seq) = seq {
                let is_array = matches!(
                    ir.resolutions.node_tys.get(&self.iterator.id()),
                    Some(Ty::Array(_))
                );
                let elem = ir.current().get_index(seq, i, AccessKind::Direct, is_array);
                let plain = matches!(
                    ir.resolutions.node_tys.get(&self.iterator.id()),
                    Some(Ty::Array(element) | Ty::Dict(element)) if element.is_plain()
                );
                owned(ir, elem, plain)
            } else {
                i
            };
            ir.current().test_pattern(&self.binding, elem, None);
        }

        ir.loop_stack_mut()
            .push(LoopCtx::new(latch, exit, collection));
        self.body.lower(ir).map(|_| ir.current().jump(latch));
        let LoopCtx {
            mut breaks,
            collection,
            ..
        } = ir.loop_stack_mut().pop().unwrap();

        ir.target(latch);
        let i = ir.current().get_local(idx_local);
        let bound = frozen_bound.unwrap_or_else(|| ir.current().len(seq.unwrap()));
        ir.current().for_next(i, bound, header);
        ir.current().jump(exit);

        breaks.push((empty, null));
        breaks.push((latch, null));
        if let Some(collection) = collection {
            ir.target(exit);
            Some(collection)
        } else {
            loop_result(ir, exit, breaks)
        }
    }
}

impl Emit for FString {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let mut parts = Vec::new();
        for part in self.parts.iter() {
            match part {
                FStringPart::Literal(literal) => {
                    let literal = ir.intern_str(literal);
                    parts.push(FormatPart::Literal(literal));
                }
                FStringPart::Expr(expr) => {
                    let value = expr.lower(ir)?;
                    parts.push(FormatPart::Value(value));
                }
            }
        }

        Some(ir.current().format(parts))
    }
}

impl Emit for parse::Function {
    fn emit(&self, id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let dec = ir.node_dec(id);
        let bid = ir.item_body_for(dec);
        ir.in_body(bid, |ir| ir.lower_fn_body(&self.parameters, &self.body));
        None
    }
}

impl Emit for parse::Impl {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        for item in &self.items {
            match item.kind() {
                parse::ItemKind::Function(f) => {
                    let _ = f.emit(item.id(), ir);
                }
                // associated consts are compile-time; nothing to lower
                parse::ItemKind::Const(_) => {}
                _ => unreachable!("parser only allows fn/const in impl bodies"),
            }
        }
        None
    }
}

impl Emit for Grouping {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        self.inner.lower(ir)
    }
}

impl Emit for Ident {
    fn emit(&self, id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let dec = ir.node_dec(id);
        match &ir.resolutions.decs[dec].kind {
            ResolvedDeclKind::Local => {
                let local = ir.local_for(dec);
                Some(ir.current().get_local(local))
            }
            ResolvedDeclKind::Item { .. } => {
                let body = ir.item_body_for(dec);
                Some(ir.current().ref_body(body))
            }
            ResolvedDeclKind::Adt(adt) => {
                let adt = *adt;
                Some(adt_value(ir, dec, adt))
            }
            ResolvedDeclKind::Pact(_) => todo!(),
            ResolvedDeclKind::Constant(_) => Constant::emit_const_dec(ir, dec),
            ResolvedDeclKind::Variant { .. } => unreachable!(
                "bare-ident reference to a variant -- variants are reached via `Foo::Bar`, not bare names"
            ),
        }
    }
}

impl Emit for If {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let then_block = ir.push_block("then");
        let else_block = ir.push_block("else");
        let merge_block = ir.push_block("merge");

        let value = self.condition.lower(ir)?;
        match self.binding.as_ref() {
            Some(binding) => ir.current().test_pattern(binding, value, Some(else_block)),
            None => {
                ir.current().jump_if_false(value, else_block);
            }
        }
        ir.current().jump(then_block);

        let then_branch = ir.in_block(then_block, |block| {
            let value = block.emit_expr(&self.main_body)?;
            let end = block.id();
            block.jump(merge_block);
            Some((end, value))
        });

        let else_branch = ir.in_block(else_block, |block| {
            let value = match self.else_expr.as_ref() {
                Some(expr) => block.emit_expr(expr)?,
                None => block.constant(Constant::Null),
            };
            let end = block.id();
            block.jump(merge_block);
            Some((end, value))
        });

        let branches: Vec<_> = [then_branch, else_branch].into_iter().flatten().collect();
        merge_branches(ir, merge_block, branches)
    }
}

impl Emit for In {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let needle = self.left.lower(ir)?;
        let target = lower_shared(ir, &self.right)?;
        Some(ir.current().check_in(needle, target, self.condition))
    }
}

impl Emit for Literal {
    fn emit(&self, id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let constant = match self {
            Literal::True => Constant::Bool(true),
            Literal::False => Constant::Bool(false),
            Literal::Null => Constant::Null,
            Literal::Unit => Constant::Null,
            Literal::String(s) => Constant::Str(ir.intern_str(s)),
            Literal::Int(i) => Constant::Int(*i),
            Literal::Float(f) => Constant::Float(*f),
            Literal::Hex(h) => Constant::Int(i64::from_str_radix(h, 16).unwrap()),
            Literal::Array(a) | Literal::Tuple(a) => {
                return ir.in_current(|block| {
                    let array = block.new_array();
                    for expr in a.iter() {
                        let value = block.emit_expr(expr)?;
                        block.push(array, value);
                    }
                    Some(array)
                });
            }
            Literal::Dictionary(fields) => {
                let dict = ir.current().new_dict();
                for (key, value) in fields.iter() {
                    let key = ir.intern_str(&key.lexeme);
                    let value = value.lower(ir)?;
                    ir.current().insert(dict, key, value);
                }
                return Some(dict);
            }
            Literal::Struct(s) => {
                let adt = match ir
                    .resolutions
                    .node_tys
                    .get(&s.name.id())
                    .or_else(|| ir.resolutions.node_tys.get(&id))
                    .unwrap()
                {
                    Ty::Adt(adt) => *adt,
                    _ => unreachable!(),
                };

                // todo, wonder if we can avoid double alloc here

                let supplied: Vec<_> = s
                    .fields
                    .iter()
                    .map(|(key, expr)| (key.to_string(), expr.lower(ir)))
                    .collect();

                let fields: Option<Vec<_>> = ir.resolutions.adts[adt]
                    .fields
                    .iter()
                    .map(|name| {
                        supplied
                            .iter()
                            .find(|(k, _)| k == name)
                            .map(|(_, v)| *v)
                            .unwrap()
                    })
                    .collect();

                let fields = fields?;

                return Some(ir.current().new_instance(adt, fields));
            }
        };

        Some(ir.current().constant(constant))
    }
}

impl Emit for Logical {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        // `&&`/`||` short-circuit: the rhs only evaluates when the lhs doesn't decide the
        // result, so its side effects must not run otherwise. the short branch's value is
        // just the lhs (false for a short `&&`, true for a short `||`).
        let rhs_block = ir.push_block("rhs");
        let short_block = ir.push_block("short");
        let merge_block = ir.push_block("merge");

        let lhs = self.left.lower(ir)?;
        let is_and = matches!(self.op, parse::LogicalOp::And);
        ir.in_current(|block| {
            if is_and {
                block.jump_if_false(lhs, short_block);
                block.jump(rhs_block);
            } else {
                block.jump_if_false(lhs, rhs_block);
                block.jump(short_block);
            }
        });

        let rhs_branch = ir.in_block(rhs_block, |block| {
            let value = block.emit_expr(&self.right)?;
            let end = block.id();
            block.jump(merge_block);
            Some((end, value))
        });

        let short_branch = ir.in_block(short_block, |block| {
            let end = block.id();
            block.jump(merge_block);
            Some((end, lhs))
        });

        let branches: Vec<_> = [rhs_branch, short_branch].into_iter().flatten().collect();
        merge_branches(ir, merge_block, branches)
    }
}

impl Emit for Loop {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let body_block = ir.push_block("body");
        let exit_block = ir.push_block("exit");
        let collection = loop_collects(&self.body).then(|| ir.current().new_array());
        ir.current().jump(body_block);

        ir.loop_stack_mut()
            .push(LoopCtx::new(body_block, exit_block, collection));

        ir.in_block(body_block, |block| {
            if block.emit_expr(&self.body).is_some() {
                block.jump(body_block);
            }
        });

        let this_loop = ir.loop_stack_mut().pop().unwrap();
        if let Some(collection) = this_loop.collection {
            ir.target(exit_block);
            Some(collection)
        } else {
            loop_result(ir, exit_block, this_loop.breaks)
        }
    }
}

impl Emit for Match {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let scrut_val = self.identity.lower(ir)?;

        if self.cases.is_empty() && !self.panic_terminator {
            return Some(ir.current().constant(Constant::Null));
        }

        let merge = ir.push_block("match_merge");
        let mut branches: Vec<(BlockId, InstId)> = vec![];

        let cases: Vec<(usize, BlockId, &MatchCase)> = self
            .cases
            .iter()
            .enumerate()
            .map(|(i, c)| (i, ir.push_block(&format!("case_{i}")), c))
            .collect();

        let fallthrough = ir.push_block("fallthrough");

        // when possible we want to avoid long chains for matches, so if our scrutinee is something
        // finite that we can reason wtih, we'll instead do a switch (jump table)
        //
        // for enums, this is pretty obvious -- one entry for each variant. but we can also do this
        // with integers. while they're not finite, the space _between_ them is, therefor our base
        // can be relative to its lowest and highest entry.
        //
        // we don't use char's, but in theory if all patterns are single char strings we could do it
        // anyway. not today though!
        //
        // many things can "go wrong" that will make us fall back to a linear chain: for
        // example, the moment an if-guard gets introduced we're (as usual) screwed. when those
        // happen we'll just bail 'plan with a None and move on to our standard chain.
        let switched: Option<(u32, Vec<BlockId>, BlockId)> = 'plan: {
            /// identifies the tag we can use to represent this variant -- be it an adt variant or
            /// an integer
            fn variant_arm(ir: &Ir, pat: &Pat) -> Option<(u32, Vec<(u32, DecId)>)> {
                let path = match pat.kind() {
                    PatKind::Literal(Literal::Int(i)) => {
                        // integers are nice and simple -- as long as they're u32 friendly, we'll
                        // take it. they have no binds to deal with
                        if (0..u32::MAX as i64).contains(i) {
                            return Some((*i as u32, vec![]));
                        }
                        return None;
                    }
                    PatKind::Variant(p) | PatKind::TupleVariant(p, _) | PatKind::Struct(p, _) => p,
                    _ => return None,
                };
                let layout = ir.layout_adt(path);
                let ident_dec = |sub: &Pat| match sub.kind() {
                    PatKind::Ident(_) => Some(ir.node_dec(sub.id())),
                    _ => None,
                };
                let mut binds = match pat.kind() {
                    PatKind::Variant(_) => vec![],
                    PatKind::TupleVariant(_, subs) => subs
                        .iter()
                        .enumerate()
                        .map(|(i, sub)| Some((i as u32, ident_dec(sub)?)))
                        .collect::<Option<Vec<_>>>()?,
                    PatKind::Struct(_, fields) => fields
                        .iter()
                        .map(|(name, sub)| {
                            Some((ir.slot(layout, &name.lexeme) as u32, ident_dec(sub)?))
                        })
                        .collect::<Option<Vec<_>>>()?,
                    _ => unreachable!(),
                };

                binds.sort_by_key(|&(slot, _)| slot);
                Some((layout.index() as u32, binds))
            }

            let mut tagged: Vec<(u32, BlockId)> = Vec::new();
            let mut default: Option<BlockId> = None;
            let mut seen = std::collections::HashSet::new();

            for (_, block, case) in &cases {
                // if we're entering another case while we've already hit a bare ident then, for
                // some reason, the user has more than one catch-all -- we can't handle that since
                // we must honor every arm, so get outta here
                if default.is_some() {
                    break 'plan None;
                }
                if case.guard().is_some() {
                    break 'plan None;
                }
                // a bare ident is the catch-all default.
                let pat = case.pat().unbound();
                if matches!(pat.kind(), PatKind::Ident(_)) {
                    default = Some(*block);
                    continue;
                }
                // otherwise one variant pattern, or the several of an `A | B`, all binding the same
                // (slot, dec) set so the single body bind serves whichever tag matched.
                let alts = match pat.kind() {
                    PatKind::Or(alts) => alts.as_slice(),
                    _ => std::slice::from_ref(pat),
                };
                let mut binds: Option<Vec<(u32, DecId)>> = None;
                for alt in alts {
                    let Some((tag, alt_binds)) = variant_arm(ir, alt.unbound()) else {
                        break 'plan None;
                    };
                    match &binds {
                        Some(b) if *b != alt_binds => break 'plan None,
                        _ => binds = Some(alt_binds),
                    }
                    if !seen.insert(tag) {
                        break 'plan None;
                    }
                    tagged.push((tag, *block));
                }
            }
            if tagged.is_empty() {
                break 'plan None;
            }

            let base = tagged.iter().map(|(t, _)| *t).min().unwrap();
            let max = tagged.iter().map(|(t, _)| *t).max().unwrap();
            let span = (max - base + 1) as usize;
            // same hueristic as rust/c to decide whether or not the table is worth it
            // if its enormous, we're going to be creating way more ops than is helpful. if its
            // big but not covering much of all the variants we have to map, its also a waste
            if span > 256 || (span <= tagged.len() * 4 && span > 64) {
                break 'plan None;
            }

            let default = default.unwrap_or(fallthrough);
            let mut table = vec![default; span];
            for (tag, block) in tagged {
                table[(tag - base) as usize] = block;
            }
            Some((base, table, default))
        };

        let is_switched = switched.is_some();
        if let Some((base, table, default)) = switched {
            ir.current().switch(scrut_val, base, table, default);
        } else {
            ir.current()
                .jump(cases.first().map_or(fallthrough, |(_, v, _)| *v));
        }

        for (i, block, case) in cases.iter() {
            let next_block = cases.get(i + 1).map_or(fallthrough, |(_, v, _)| *v);

            // a body that terminates (`return`, `break`, ...) has nothing to feed into
            // the phi at `merge` -- skip pushing it as a branch and move on to the next
            // arm. propagating the None here would abort emit for every later arm.
            let branch = ir.in_block(*block, |b| {
                // the switch already proved the tag
                let fail_block = (!is_switched).then_some(next_block);
                b.test_pattern(case.pat(), scrut_val, fail_block);
                if let Some(guard) = case.guard() {
                    let guard_val = b.emit_expr(guard)?;
                    b.jump_if_false(guard_val, next_block);
                }

                let body = b.emit_expr(case.body())?;
                let end = b.id();
                b.jump(merge);

                Some((end, body))
            });

            if let Some(pair) = branch {
                branches.push(pair);
            }
        }

        ir.in_block(fallthrough, |b| {
            if self.panic_terminator {
                b.panic();
            } else {
                let null = b.constant(Constant::Null);
                let end = b.id();
                b.jump(merge);
                branches.push((end, null));
            }
        });

        merge_branches(ir, merge, branches)
    }
}

impl Emit for Return {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let value = match &self.value {
            Some(expr) => expr.lower(ir)?,
            None => ir.current().constant(Constant::Null),
        };
        ir.current().ret(value);
        None
    }
}

impl Emit for Unary {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let right = self.right.lower(ir)?;
        Some(ir.current().unary(self.op.into(), right))
    }
}

impl Emit for Unwrap {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let value = self.expr.lower(ir)?;
        Some(ir.current().unwrap(value))
    }
}

impl Emit for Raise {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let value = self.value.lower(ir)?;
        ir.current().raise(value);
        None
    }
}

impl Emit for Absolve {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let value = self.left.lower(ir)?;
        let handler = self.handler.lower(ir)?;

        let err_block = ir.push_block("absolve_err");
        let ok_block = ir.push_block("absolve_ok");
        let merge = ir.push_block("absolve_merge");

        let is_raised = ir.current().is_raised(value);
        ir.in_current(|b| {
            b.jump_if_false(is_raised, ok_block);
            b.jump(err_block);
        });

        let err_branch = ir.in_block(err_block, |b| {
            let err = b.unwrap_raised(value);
            let r = b.call(handler, vec![err]);
            let end = b.id();
            b.jump(merge);
            (end, r)
        });

        let ok_end = ir.in_block(ok_block, |b| {
            let end = b.id();
            b.jump(merge);
            end
        });

        ir.target(merge);
        Some(ir.current().phi(vec![err_branch, (ok_end, value)]))
    }
}

impl Emit for While {
    fn emit(&self, _id: NodeId, ir: &mut Ir) -> Option<InstId> {
        let condition_block = ir.push_block("condition");
        let body_block = ir.push_block("body");
        let condition_exit = ir.push_block("condition_exit");
        let exit = ir.push_block("exit");

        let collection = loop_collects(&self.body).then(|| ir.current().new_array());
        ir.current().jump(condition_block);

        ir.loop_stack_mut()
            .push(LoopCtx::new(condition_block, exit, collection));

        let condition_reachable = ir.in_block(condition_block, |block| {
            let value = block.emit_expr(&self.header)?;
            match self.binding.as_ref() {
                Some(binding) => {
                    block.test_pattern(binding, value, Some(condition_exit));
                }
                None => {
                    block.jump_if_false(value, condition_exit);
                }
            }
            block.jump(body_block);
            Some(())
        });

        ir.in_block(body_block, |block| {
            if block.emit_expr(&self.body).is_some() {
                block.jump(condition_block);
            }
        });

        let normal_exit = condition_reachable.map(|()| {
            ir.in_block(condition_exit, |block| {
                let value = block.constant(Constant::Null);
                block.jump(exit);
                (condition_exit, value)
            })
        });

        let LoopCtx {
            mut breaks,
            collection,
            ..
        } = ir.loop_stack_mut().pop().unwrap();
        breaks.extend(normal_exit);

        if let Some(collection) = collection {
            ir.target(exit);
            Some(collection)
        } else {
            loop_result(ir, exit, breaks)
        }
    }
}

fn emit_bin_op(
    ir: &mut Ir,
    id: NodeId,
    left: &Expr,
    op: impl Into<BinOp>,
    right: &Expr,
) -> Option<InstId> {
    let kind = num_kind(ir, id, left, right);
    let left = left.lower(ir)?;
    let right = right.lower(ir)?;
    Some(ir.current().bin(op.into(), left, right, kind))
}

fn emit_coalesce(ir: &mut Ir, lhs: InstId, rhs: &Expr) -> Option<InstId> {
    // `??` short-circuits: the rhs only evaluates when the lhs is null. a non-null option's runtime
    // value IS its inner value, so the keep branch passes the lhs straight through.
    let rhs_block = ir.push_block("rhs");
    let keep_block = ir.push_block("keep");
    let merge_block = ir.push_block("merge");

    ir.in_current(|block| {
        block.jump_if_null(lhs, rhs_block);
        block.jump(keep_block);
    });

    let rhs_branch = ir.in_block(rhs_block, |block| {
        let value = block.emit_expr(rhs)?;
        let end = block.id();
        block.jump(merge_block);
        Some((end, value))
    });

    let keep_branch = ir.in_block(keep_block, |block| {
        let end = block.id();
        block.jump(merge_block);
        Some((end, lhs))
    });

    let branches: Vec<_> = [rhs_branch, keep_branch].into_iter().flatten().collect();
    merge_branches(ir, merge_block, branches)
}

fn num_kind(ir: &Ir, id: NodeId, left: &Expr, right: &Expr) -> OperandKind {
    match num_kind_of(ir, id) {
        // a Bool result is a comparison/equality (`==`, `<`, ...); the specialization comes from
        // the operands (int/float/str/bool), not the always-Bool result, so fall through
        // like Generic.
        OperandKind::Generic | OperandKind::Bool => {
            let l = num_kind_of(ir, left.id());
            if l != OperandKind::Generic {
                l
            } else {
                num_kind_of(ir, right.id())
            }
        }
        k => k,
    }
}
fn num_kind_of(ir: &Ir, id: NodeId) -> OperandKind {
    match ir.resolutions.node_tys.get(&id) {
        Some(Ty::Bool) => OperandKind::Bool,
        Some(Ty::Int) => OperandKind::Int,
        Some(Ty::Float) => OperandKind::Float,
        Some(Ty::Str) => OperandKind::Str,
        _ => OperandKind::Generic,
    }
}

// a struct's name used as a value (the struct itself when it has no fields, otherwise its ctor)
fn adt_value(ir: &mut Ir, dec: DecId, adt: AdtId) -> InstId {
    if ir.resolutions.adts[adt].fields.is_empty() {
        return ir.current().new_instance(adt, vec![]);
    }
    let body = ir.item_body_for(dec);
    ir.current().ref_body(body)
}

// mutable parts taken out of a shared constant need their own copy
fn owned(ir: &mut Ir, part: InstId, plain: bool) -> InstId {
    if plain || !is_shared(ir, part) {
        part
    } else {
        ir.current().deep_clone(part)
    }
}

fn is_shared(ir: &Ir, value: InstId) -> bool {
    match ir.current_body().instructions[value] {
        Inst::Shared(_) => true,
        Inst::GetIndex { set: from, .. } | Inst::GetField { src: from, .. } => is_shared(ir, from),
        _ => false,
    }
}

// this use only inspects the value
fn lower_shared(ir: &mut Ir, expr: &Expr) -> Option<InstId> {
    match expr.kind() {
        ExprKind::Grouping(grouping) => lower_shared(ir, &grouping.inner),
        ExprKind::Ident(_) | ExprKind::Access(Access::DoubleColon { .. }) => {
            let dec = ir.node_dec(expr.id());
            match &ir.resolutions.decs[dec].kind {
                // a folded scalar or string has nothing to share
                ResolvedDeclKind::Constant(
                    ConstValue::Expr(_)
                    | ConstValue::Literal(Literal::Array(_) | Literal::Tuple(_)),
                ) => {
                    let slot = ir.shared_for(dec);
                    Some(ir.current().shared(slot))
                }
                _ => expr.lower(ir),
            }
        }
        // `owned` added a clone for a part that this use only inspects
        _ => {
            let value = expr.lower(ir)?;
            Some(match ir.current_body().instructions[value] {
                Inst::DeepClone(part) => part,
                _ => value,
            })
        }
    }
}

// null when `receiver` is, and what `then` gives when it isn't
fn unless_null(
    ir: &mut Ir,
    receiver: InstId,
    then: impl FnOnce(&mut Ir) -> Option<InstId>,
) -> Option<InstId> {
    let null_lit = ir.current().constant(Constant::Null);
    let is_null = ir
        .current()
        .bin(BinOp::Identity, receiver, null_lit, OperandKind::Generic);
    let some_block = ir.push_block("opt_some");
    let null_block = ir.push_block("opt_null");
    let merge = ir.push_block("opt_merge");

    ir.in_current(|block| {
        block.jump_if_false(is_null, some_block);
        block.jump(null_block);
    });

    ir.target(null_block);
    let null_value = ir.current().constant(Constant::Null);
    let null_end = ir.current_block_id();
    ir.current().jump(merge);

    ir.target(some_block);
    let value = then(ir)?;
    let end = ir.current_block_id();
    ir.current().jump(merge);

    merge_branches(ir, merge, vec![(null_end, null_value), (end, value)])
}

// each implementer of every pact in `pids`, as its `member` and the ids its values carry
fn pact_candidates<T>(
    ir: &Ir,
    pids: &[PactId],
    member: impl Fn(&ResolvedAdt) -> T,
) -> Vec<(T, Vec<AdtId>)> {
    ir.resolutions
        .adts
        .iter()
        .filter(|(_, adt)| pids.iter().all(|p| adt.implements.contains(p)))
        .map(|(_, adt)| (member(adt), adt.dispatch_ids.clone()))
        .collect()
}

// a runtime IsInstance chain on `recv`, where the arm of the implementer it turns out to be gives
// the value
fn pact_dispatch<T: Copy>(
    ir: &mut Ir,
    recv: InstId,
    candidates: Vec<(T, Vec<AdtId>)>,
    mut arm: impl FnMut(&mut Ir, T) -> Option<InstId>,
) -> Option<InstId> {
    let merge = ir.push_block("pact_merge");
    let mut branches = Vec::new();
    for (member, ids) in candidates {
        for id in ids {
            let hit = ir.push_block("pact_hit");
            let next = ir.push_block("pact_next");
            let is_inst = ir.current().is_instance(recv, id);
            ir.in_current(|block| {
                block.jump_if_false(is_inst, next);
                block.jump(hit);
            });
            ir.target(hit);
            let value = arm(ir, member)?;
            let end = ir.current_block_id();
            ir.current().jump(merge);
            branches.push((end, value));
            ir.target(next);
        }
    }
    // only an implementor added after this site was compiled gets here
    ir.current().no_impl();
    merge_branches(ir, merge, branches)
}

fn merge_branches(
    ir: &mut Ir,
    merge_block: BlockId,
    branches: Vec<(BlockId, InstId)>,
) -> Option<InstId> {
    if branches.is_empty() {
        return None;
    }
    ir.target(merge_block);
    Some(ir.current().phi(branches))
}

fn loop_result(ir: &mut Ir, exit: BlockId, branches: Vec<(BlockId, InstId)>) -> Option<InstId> {
    ir.target(exit);
    Some(ir.current().phi(branches))
}

fn loop_collects(expr: &Expr) -> bool {
    match expr.kind() {
        ExprKind::Collect(_) => true,
        ExprKind::Block(block) => {
            block.body.iter().any(|stmt| match stmt.kind() {
                StmtKind::Expr(expr) => loop_collects(expr),
                _ => false,
            }) || block.yielded_expr.as_ref().is_some_and(loop_collects)
        }
        ExprKind::If(if_expr) => {
            loop_collects(&if_expr.main_body)
                || if_expr.else_expr.as_ref().is_some_and(loop_collects)
        }
        ExprKind::Loop(_) | ExprKind::While(_) | ExprKind::For(_) => false,
        _ => false,
    }
}

/// Conservative guard for the Len-hoist: true if the loop body might change the iterated
/// collection's length. We currently bail on _any_ call, which is enormously limiting, but it's a
/// pretty big lift to do more intelligent scanning and the performance we're leaving on the table
/// is real, but not worth this much work in the compiler (at least for now).
fn body_may_mutate_len(expr: &Expr, dict_iter: bool) -> bool {
    let see = |e| body_may_mutate_len(e, dict_iter);
    match expr.kind() {
        ExprKind::Call(_) => true,
        ExprKind::Block(b) => {
            b.body.iter().any(|s| stmt_may_mutate_len(s, dict_iter))
                || b.yielded_expr.as_ref().is_some_and(see)
        }
        ExprKind::If(i) => {
            see(&i.condition) || see(&i.main_body) || i.else_expr.as_ref().is_some_and(see)
        }
        ExprKind::In(i) => see(&i.left) || see(&i.right),
        ExprKind::Evaluation(e) => see(&e.left) || see(&e.right),
        ExprKind::Equality(e) => see(&e.left) || see(&e.right),
        ExprKind::Logical(l) => see(&l.left) || see(&l.right),
        ExprKind::Coalescence(c) => see(&c.left) || see(&c.right),
        ExprKind::Unary(u) => see(&u.right),
        ExprKind::Unwrap(u) => see(&u.expr),
        ExprKind::Grouping(g) => see(&g.inner),
        ExprKind::Access(Access::Dot { left, right, .. }) => see(left) || see(right),
        ExprKind::Access(Access::Square { left, key, .. }) => see(left) || see(key),
        ExprKind::Ident(_) | ExprKind::Literal(_) => false,
        _ => true,
    }
}

fn stmt_may_mutate_len(stmt: &Stmt, dict_iter: bool) -> bool {
    let see = |e| body_may_mutate_len(e, dict_iter);
    match stmt.kind() {
        StmtKind::Expr(e) => see(e),
        StmtKind::Let(l) => see(&l.right),
        StmtKind::Assignment(a) => {
            // `d[newkey] = v` grows a dict's length (unlike array index-assignment, which is
            // length-preserving), so when iterating a dict any indexed assignment may mutate len.
            (dict_iter && matches!(a.left.kind(), ExprKind::Access(Access::Square { .. })))
                || see(&a.left)
                || see(&a.right)
        }
        _ => true,
    }
}
