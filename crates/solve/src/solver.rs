use crate::{
    Error, Result, Unification, UnificationError,
    components::*,
    errors::{
        AssignToConst, AssignToLoopVar, AssignToStringIndex, BareNullBinding, ExtraTupleMembers,
        FieldNotFound, InvalidAssignTarget, InvalidPattern, InvalidUseTarget, MissingTupleMembers,
        MultipleConstDeclarations, NonConstantValue, NotFound, SelfOutOfContext,
    },
    traits::*,
};
use api::{ApiAdt, ApiAdtKind, ApiEntry, ApiVariantFields, Library, NativeId};
use indexmap::{IndexMap, IndexSet};
use miette::NamedSource;
use parse::{
    ExprKind,
    components::{Binding, Pat, PatKind},
    *,
};
use shared::{FileId, IdVec, Literal as RawLiteral, Located, Location, PactId};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    sync::Arc,
};

pub struct Solver {
    pub(crate) decs: IdVec<DecId, Dec>,
    pub(crate) adts: IdVec<AdtId, Adt>,
    pub(crate) pacts: IdVec<PactId, shared::Pact>,
    pub(crate) pact_impls: HashSet<(PactId, AdtId)>,
    pub(crate) pact_default_decs: HashMap<(PactId, String), DecId>,
    pub(crate) pact_self: Option<PactId>,
    pub(crate) node_to_vid: IndexMap<NodeId, Vid>,
    pub(crate) node_decs: IndexMap<NodeId, DecId>,
    pub(crate) closure_captures: IndexMap<NodeId, IndexSet<DecId>>,
    pub(crate) expected_closures: IndexMap<NodeId, Vec<FnParam>>,
    vids: IdVec<Vid, ()>,
    subs: IdVec<Vid, Option<Ty>>,
    node_visits: HashSet<NodeId>,
    library: HashMap<String, AdtId>,
    sources: HashMap<FileId, NamedSource<Arc<str>>>,

    pub(crate) dec_to_native: HashMap<DecId, NativeBinding>,

    pub(crate) control_flow: ControlFlow,
    pub(crate) ribs: Ribs,
    pub(crate) loop_stack: Vec<LoopRun>,
    pub(crate) fn_stack: Vec<FnRun>,
}

// Public impls
impl Solver {
    pub fn new() -> Self {
        let mut solver = Self {
            vids: IdVec::new(),
            subs: IdVec::new(),
            decs: IdVec::new(),
            adts: IdVec::new(),
            pacts: IdVec::new(),
            pact_impls: HashSet::new(),
            pact_default_decs: HashMap::new(),
            pact_self: None,
            node_to_vid: IndexMap::new(),
            node_decs: IndexMap::new(),
            closure_captures: IndexMap::new(),
            expected_closures: IndexMap::new(),
            control_flow: ControlFlow::new(),
            node_visits: HashSet::new(),
            ribs: Ribs::default(),
            loop_stack: vec![],
            fn_stack: vec![],
            library: HashMap::new(),
            sources: HashMap::new(),
            dec_to_native: HashMap::new(),
        };
        solver.ribs.push_import();
        solver.ribs.push_block();

        // todo, this is a bit brittle
        for name in ["int", "float", "str", "bool", "array", "dict"] {
            let mut adt = Adt::new_struct(name.into());
            adt.flags |= AdtFlags::IS_BUILTIN;
            let id = solver.push_adt(adt);
            solver.library.insert(name.into(), id);
        }
        solver
    }

    /// Install the per-file source map for diagnostic rendering. Call before `solve_all`.
    pub fn set_sources(&mut self, sources: HashMap<FileId, NamedSource<Arc<str>>>) {
        self.sources = sources;
    }

    /// Look up the `NamedSource` for a `Location`'s file. Falls back to an empty `<unknown>` source
    /// if the file isn't in the map. This is for the sake of tests, we could use cfgs and be more
    /// strict if we wanted
    pub(crate) fn src(&self, loc: Location) -> NamedSource<Arc<str>> {
        self.sources
            .get(&loc.file_id)
            .cloned()
            .unwrap_or_else(|| NamedSource::new("<unknown>", Arc::<str>::from("")))
    }

    /// Cross-module visibility check. A `Private` dec whose home module differs from the
    /// one currently being solved is unreachable -- errors at the access location. Decs
    /// homed at `AdtId::DANGLING` (builtins, natives, locals) are always reachable.
    pub(crate) fn check_vis(&self, dec: DecId, access_at: Location) -> Result<()> {
        let dec = &self.decs[dec];
        if dec.vis == Vis::Private
            && dec.module != AdtId::DANGLING
            && dec.module != self.ribs.current_module()
        {
            Err(crate::errors::PrivateAccess {
                src: self.src(access_at),
                name: dec.name.clone(),
                access_at: access_at.into(),
                declaration: vec![crate::errors::PrivateDeclaredHere {
                    src: self.src(dec.location),
                    at: dec.location.into(),
                    name: dec.name.clone(),
                }],
            })?
        }
        Ok(())
    }

    pub(crate) fn builtin_adt(&self, ty: &Ty) -> Option<AdtId> {
        let name = match ty {
            Ty::Int => "int",
            Ty::Float => "float",
            Ty::Str => "str",
            Ty::Bool => "bool",
            Ty::Array(_) => "array",
            Ty::Dict(_) => "dict",
            _ => return None,
        };
        self.library.get(name).copied()
    }

    pub fn solve(&mut self, ast: &Ast) -> Result<()> {
        self.solve_all([ast])
    }

    pub fn solve_all<'a>(&mut self, asts: impl IntoIterator<Item = &'a Ast>) -> Result<()> {
        let asts: Vec<_> = asts.into_iter().collect();
        let module_names: Vec<Option<String>> = asts
            .iter()
            .map(|ast| ast.module_name())
            .collect::<Result<_>>()?;

        // pre-allocate one Adt per named module so cross-file refs can resolve at hoist time.
        // nested decls (`module a::b;`) build the parent chain; the file's rib binds the leaf
        let mut module_adts: HashMap<String, AdtId> = HashMap::new();
        for name in module_names.iter().flatten() {
            let segments: Vec<String> = name.split("::").map(str::to_string).collect();
            let id = self.ensure_module_path(&segments);
            module_adts.insert(name.clone(), id);
        }

        // each ast's module rib accumulates across phases -- saved between visits
        let mut saved_ribs: Vec<Option<Rib>> = vec![None; asts.len()];

        let run_phase = |solver: &mut Self,
                         phase: fn(&mut Self, &Ast) -> Result<()>,
                         saved_ribs: &mut Vec<Option<Rib>>|
         -> Result<()> {
            for (i, ast) in asts.iter().enumerate() {
                let module = module_names[i].as_ref();
                if let Some(name) = module {
                    let rib = saved_ribs[i]
                        .take()
                        .unwrap_or_else(|| Rib::new(RibKind::Module(module_adts[name])));
                    solver.ribs.push_rib(rib);
                    solver.ribs.push_import();
                    solver.ribs.push_block();
                }
                phase(solver, ast)?;
                if let Some(name) = module {
                    solver.ribs.pop_block();
                    solver.ribs.pop_import();
                    let rib = solver.ribs.pop_module();
                    solver.sync_module_adt(module_adts[name], &rib);
                    saved_ribs[i] = Some(rib);
                }
            }
            Ok(())
        };

        run_phase(self, Self::hoist_types, &mut saved_ribs)?;
        run_phase(self, Self::hoist_pacts, &mut saved_ribs)?;
        run_phase(self, Self::hoist_callables, &mut saved_ribs)?;
        run_phase(self, Self::hoist_constants, &mut saved_ribs)?;

        // fixpoint: hoist_uses + solve_consts. each round may resolve more consts (e.g. a const
        // that references another module's const reduces only once that other const is bound).
        // re-run while the count of consts-with-known-values grows, capped as a safety net.
        const FIXPOINT_CAP: usize = 16;
        let mut prev = self.resolved_constants();
        for _ in 0..FIXPOINT_CAP {
            run_phase(self, Self::hoist_uses, &mut saved_ribs)?;
            run_phase(self, Self::solve_consts, &mut saved_ribs)?;
            let curr = self.resolved_constants();
            if curr == prev {
                break;
            }
            prev = curr;
        }

        run_phase(self, Self::solve_types, &mut saved_ribs)?;
        run_phase(self, Self::solve_bodies, &mut saved_ribs)?;

        // body solving is the last chance for an impl-associated or block-scoped const to gain
        // its literal value. anything still at `Constant(None)` afterwards is unresolvable --
        // almost always a self/mutual cycle. surface the first such dec so it doesn't reach IR.
        if let Some((_, dec)) = self
            .decs
            .iter()
            .find(|(_, d)| matches!(d.kind, DecKind::Constant(None)))
        {
            let location = dec.location;
            let name = dec.name.clone();
            Err(crate::errors::UnresolvedConst {
                src: self.src(location),
                at: location.into(),
                name,
            })?;
        }

        self.control_flow = ControlFlow::default();
        assert!(self.fn_stack.is_empty());
        assert!(self.loop_stack.is_empty());
        Ok(())
    }

    fn for_each_item<F>(ast: &Ast, mut f: F) -> Result<()>
    where
        F: FnMut(&Item) -> Result<()>,
    {
        ast.stmts().iter().try_for_each(|stmt| {
            let StmtKind::Item(item) = stmt.kind() else {
                return Ok(());
            };
            f(item)
        })
    }

    fn hoist_pacts(&mut self, ast: &Ast) -> Result<()> {
        Self::for_each_item(ast, |item| {
            let ItemKind::Pact(p) = item.kind() else {
                return Ok(());
            };
            let vis = if item.public() {
                Vis::Public
            } else {
                Vis::Private
            };
            p.hoist(HoistCtx::new(
                self,
                HoistTarget::Scope,
                Some(item.id()),
                item.location(),
                vis,
            ))
        })
    }

    fn hoist_types(&mut self, ast: &Ast) -> Result<()> {
        Self::for_each_item(ast, |item| {
            let vis = if item.public() {
                Vis::Public
            } else {
                Vis::Private
            };
            let ctx = HoistCtx::new(
                self,
                HoistTarget::Scope,
                Some(item.id()),
                item.location(),
                vis,
            );
            match item.kind() {
                ItemKind::Struct(s) => s.hoist(ctx),
                ItemKind::Enum(e) => e.hoist(ctx),
                _ => Ok(()),
            }
        })
    }

    fn hoist_callables(&mut self, ast: &Ast) -> Result<()> {
        Self::for_each_item(ast, |item| {
            if let ItemKind::Use(us) = item.kind() {
                let _ = self.process_use(us, item.location());
            }
            Ok(())
        })?;
        Self::for_each_item(ast, |item| {
            let vis = if item.public() {
                Vis::Public
            } else {
                Vis::Private
            };
            let ctx = HoistCtx::new(
                self,
                HoistTarget::Scope,
                Some(item.id()),
                item.location(),
                vis,
            );
            match item.kind() {
                ItemKind::Impl(i) => i.hoist(ctx),
                ItemKind::Function(f) => f.hoist(ctx),
                _ => Ok(()),
            }
        })
    }

    fn hoist_constants(&mut self, ast: &Ast) -> Result<()> {
        Self::for_each_item(ast, |item| {
            let ItemKind::Const(con) = item.kind() else {
                return Ok(());
            };
            let vis = if item.public() {
                Vis::Public
            } else {
                Vis::Private
            };
            con.hoist(HoistCtx::new(
                self,
                HoistTarget::Scope,
                Some(item.id()),
                item.location(),
                vis,
            ))
        })
    }

    fn hoist_uses(&mut self, ast: &Ast) -> Result<()> {
        Self::for_each_item(ast, |item| {
            let ItemKind::Use(us) = item.kind() else {
                return Ok(());
            };
            self.process_use(us, item.location())
        })
    }

    fn solve_bodies(&mut self, ast: &Ast) -> Result<()> {
        ast.stmts().iter().try_for_each(|v| self.visit_stmt(v))
    }

    fn solve_types(&mut self, ast: &Ast) -> Result<()> {
        Self::for_each_item(ast, |item| {
            if let ItemKind::Use(us) = item.kind() {
                self.process_use(us, item.location())?;
            }
            Ok(())
        })?;
        Self::for_each_item(ast, |item| {
            if !matches!(item.kind(), ItemKind::Struct(_) | ItemKind::Enum(_))
                || !self.touch_node(item.id())
            {
                return Ok(());
            }
            let ty = match item.kind() {
                ItemKind::Struct(s) => s.solve(item.id(), item.location(), self)?,
                ItemKind::Enum(e) => e.solve(item.id(), item.location(), self)?,
                _ => unreachable!(),
            };
            let vid = self.node_vid(item.id());
            self.register_sub(vid, ty)
                .map_err(|e| e.into_type_mismatch(self, item.location()))
        })
    }

    fn solve_consts(&mut self, ast: &Ast) -> Result<()> {
        Self::for_each_item(ast, |item| {
            if let ItemKind::Const(con) = item.kind() {
                self.solve_const(con, item.id()).map(|_| ())
            } else {
                Ok(())
            }
        })
    }

    /// Number of `Constant` decls whose literal value has been reduced. Used by the hoist-uses /
    /// solve-consts fixpoint loop to detect "no progress this round -> stop."
    fn resolved_constants(&self) -> usize {
        self.decs
            .iter()
            .filter(|(_, d)| matches!(d.kind, DecKind::Constant(Some(_))))
            .count()
    }

    /// Solve a single `const` item's rhs and, if reducible, populate `DeclKind::Constant`'s
    /// payload with the literal. No-op for non-const items. Shared by the top-level fixpoint,
    /// impl associated consts, and block-scoped consts during body visits.
    pub(crate) fn solve_const(&mut self, con: &Const, id: NodeId) -> Result<Ty> {
        let Const {
            left,
            annotation,
            right,
        } = con;

        let vid = self.node_vid(right.id());
        let mut ty = Ty::Vid(vid);

        if let Some(annotation) = &annotation {
            ty.fulfill_ty(&mut Ty::from_annotation(annotation.clone(), self)?, self)
                .map_err(|e| e.into_type_mismatch(self, left.location()))?;
        };

        // solve rhs first so inner idents land in `node_decs` before reduce_const_expr uses them
        right.fulfill_ty(ty.clone(), self)?;

        if annotation.is_none() && ty.clone().normalized(self) == Ty::Null {
            Err(BareNullBinding {
                src: self.src(right.location()),
                at: right.location().into(),
            })?;
        }

        if !self.is_const_expr(right) {
            Err(NonConstantValue {
                src: self.src(right.location()),
                at: right.location().into(),
                value: crate::errors::elide(right.to_string()),
            })?;
        }

        if let (Some(lit), Some(dec)) = (
            self.reduce_const_expr(right)?,
            self.node_decs.get(&id).copied(),
        ) && let DecKind::Constant(slot) = &mut self.decs[dec].kind
        {
            *slot = Some(lit);
        }

        Ok(ty)
    }

    fn process_use(&mut self, us: &Use, location: Location) -> Result<()> {
        let walk_segments: Vec<&Ident> = match us {
            Use::Singular(path, item) => path.iter().chain(std::iter::once(item)).collect(),
            Use::Multi(path, _) | Use::All(path) => path.iter().collect(),
        };

        let first = *walk_segments.first().unwrap();
        let mut target = ImportBinding {
            name: first.lexeme.clone(),
            ty: self
                .library
                .get(&first.lexeme)
                .cloned()
                .map(Ty::Adt)
                .ok_or_else(|| NotFound {
                    src: self.src(first.location),
                    at: first.location.into(),
                    name: first.lexeme.clone(),
                })?,
            dec: None,
            constant: false,
        };

        for ident in walk_segments.iter().skip(1) {
            let Some(adt) = target.ty.as_adt().copied() else {
                Err(InvalidUseTarget {
                    src: self.src(ident.location),
                    at: ident.location.into(),
                })?
            };
            target = self.module_field(adt, ident)?;
        }

        let imports: Vec<ImportBinding> = match us {
            Use::Singular(_, _) => vec![target],
            Use::Multi(_, items) => {
                let Some(adt) = target.ty.as_adt().copied() else {
                    Err(InvalidUseTarget {
                        src: self.src(location),
                        at: location.into(),
                    })?
                };
                items
                    .iter()
                    .map(|i| self.module_field(adt, i))
                    .collect::<Result<Vec<_>>>()?
            }
            Use::All(_) => {
                let Some(adt) = target.ty.as_adt().copied() else {
                    Err(InvalidUseTarget {
                        src: self.src(location),
                        at: location.into(),
                    })?
                };
                let module = &self.adts[adt];
                if !module.flags.contains(AdtFlags::IS_MODULE) {
                    Err(InvalidUseTarget {
                        src: self.src(location),
                        at: location.into(),
                    })?
                }
                module
                    .as_struct()
                    .fields
                    .iter()
                    .map(|(name, field)| ImportBinding {
                        name: name.clone(),
                        ty: field.ty.clone(),
                        dec: Some(field.dec),
                        constant: field.constant,
                    })
                    .chain(module.impls.iter().map(|(name, field)| ImportBinding {
                        name: name.clone(),
                        ty: field.ty.clone(),
                        dec: Some(field.dec),
                        constant: field.constant,
                    }))
                    .collect()
            }
        };

        for import in imports {
            self.declare_import(import)?;
        }
        Ok(())
    }

    /// Pre-binds every `const` declared directly inside this block into the current rib so refs
    /// above the declaration line resolve, matching Rust's scope-hoisted-item semantics. Walks one
    /// level -- nested blocks hoist themselves when their own `Block::solve` runs.
    pub(crate) fn hoist_block_consts(&mut self, body: &[Stmt]) {
        for stmt in body {
            let StmtKind::Item(item) = stmt.kind() else {
                continue;
            };
            let ItemKind::Const(Const { left, right, .. }) = item.kind() else {
                continue;
            };
            let vid = self.node_vid(right.id());
            let vis = if item.public() {
                Vis::Public
            } else {
                Vis::Private
            };
            let dec = self.dec_id(left, Ty::Vid(vid), DecKind::Constant(None), vis);
            self.node_decs.insert(item.id(), dec);
            self.ribs.current_mut().insert(left.clone(), dec);
        }
    }

    // every adt allocation routes through here so `Display for Ty` can name it (the
    // shared-crate name table is what stands between users and `<adt>` placeholders)
    pub(crate) fn push_adt(&mut self, adt: Adt) -> AdtId {
        let name = adt.name.clone();
        let id = self.adts.push(adt);
        shared::name_adt(id.index(), &name);
        id
    }

    fn sync_module_adt(&mut self, adt: AdtId, module: &Rib) {
        let mut entries: Vec<_> = module
            .table
            .iter()
            .map(|(name, dec)| (name.clone(), *dec))
            .collect();
        entries.sort_by(|(a, _), (b, _)| a.cmp(b));

        // child modules (`module a::b;` registers `b` inside `a` at pre-alloc) live only in the
        // fields, not in any file's rib -- they must survive the rebuild
        let children: Vec<_> = self.adts[adt]
            .as_struct()
            .fields
            .iter()
            .filter(|(_, f)| {
                f.ty.as_adt()
                    .is_some_and(|id| self.adts[id].flags.contains(AdtFlags::IS_MODULE))
            })
            .map(|(n, f)| (n.clone(), f.clone()))
            .collect();
        self.adts[adt].as_struct_mut().fields.clear();
        for (name, field) in children {
            self.adts[adt].as_struct_mut().insert(name, field);
        }
        for (name, did) in entries {
            let dec = &self.decs[did];
            // store the bare Vid -- not its current substitution. readers always normalize, so
            // they pick up whatever the vid resolves to AT READ TIME. avoids snapshotting an
            // unresolved type when a later-phase or later-file body still has to bind it.
            let ty = Ty::Vid(dec.vid);
            self.adts[adt].as_struct_mut().insert(
                name,
                Field {
                    ty,
                    constant: dec.kind.is_constant(),
                    declaration_location: dec.location,
                    dec: did,
                },
            );
        }
    }

    fn module_field(&self, adt: AdtId, ident: &Ident) -> Result<ImportBinding> {
        let module = &self.adts[adt];
        if !module.flags.contains(AdtFlags::IS_MODULE) {
            Err(InvalidUseTarget {
                src: self.src(ident.location),
                at: ident.location.into(),
            })?
        }

        let field = module
            .as_struct()
            .fields
            .get(&ident.lexeme)
            .or_else(|| module.impls.get(&ident.lexeme))
            .cloned()
            .ok_or_else(|| FieldNotFound {
                src: self.src(ident.location),
                at: ident.location.into(),
                field_name: ident.lexeme.clone(),
            })?;

        Ok(ImportBinding {
            name: ident.lexeme.clone(),
            ty: field.ty,
            dec: Some(field.dec),
            constant: field.constant,
        })
    }

    pub fn declare_native_fn(
        &mut self,
        name: String,
        params: Vec<Option<Ty>>,
        return_ty: Option<Ty>,
        native_id: NativeId,
    ) -> DecId {
        let sig = NativeFnSig {
            params,
            return_ty,
            recv: None,
        };
        // first instantiation fills the dec's vid, later ident resolutions detect that this is
        // a native and re-instantiate so each call gets its own type vars.
        let initial = self
            .instantiate_native(&sig, None)
            .expect("free fn has no recv");
        let ident = Ident::synthetic(name);
        let dec_id = self.dec_id(
            &ident,
            initial,
            DecKind::Item { defaults: vec![] },
            Vis::Public,
        );
        self.ribs.module_mut().insert(ident, dec_id);
        self.dec_to_native
            .insert(dec_id, NativeBinding { id: native_id, sig });
        dec_id
    }

    /// Returns the `Ty::Fn` for a native callable, with every `Ty::Anon(N)` in the sig replaced
    /// by a fresh `Ty::Vid` (same `N` -> same vid, within this one call). If a receiver pattern
    /// is present and `recv_check` is provided, the actual receiver type is unified against the
    /// pattern -- failure means the method isn't applicable to this receiver.
    pub(crate) fn instantiate_native(
        &mut self,
        sig: &NativeFnSig,
        recv_check: Option<(&Ty, Location)>,
    ) -> Result<Ty> {
        let mut memo: HashMap<u32, Vid> = HashMap::new();
        let materialized: Vec<Ty> = sig
            .params
            .iter()
            .map(|p| self.materialize_slot(p, &mut memo))
            .collect();
        // trailing Option params are omittable: marking them defaulted lets the call-site
        // arity check pass, and the IR fills the empty slot with null (-> None natively)
        let mut optional_from = materialized.len();
        while optional_from > 0 && matches!(materialized[optional_from - 1], Ty::Option(_)) {
            optional_from -= 1;
        }
        let parameters: Vec<FnParam> = materialized
            .into_iter()
            .enumerate()
            .map(|(i, ty)| FnParam::new(None, ty, i >= optional_from))
            .collect();
        let return_ty = self.materialize_slot(&sig.return_ty, &mut memo);

        if let (Some(recv_pat), Some((actual, location))) = (sig.recv.as_ref(), recv_check) {
            let mut pat = self.substitute_anons(recv_pat, &mut memo);
            let mut actual = actual.clone();
            actual
                .fulfill_ty(&mut pat, self)
                .map_err(|e| e.into_type_mismatch(self, location))?;
        }

        Ok(Ty::Fn(FnHeader::new(parameters, return_ty, false)))
    }

    /// Produce a fresh `Ty::Fn` from a pact's stored header so unifying it against an impl can't
    /// pollute the shared signature. A pact header holds vids minted once at hoist (notably the
    /// `self` slot, left unbound); without freshening, the first `impl Pact for A` binds those
    /// vids globally and a later `impl Pact for B` would unify against the stale binding.
    pub(crate) fn instantiate_pact_fn(&mut self, header: &FnHeader) -> Ty {
        let mut memo: HashMap<Vid, Vid> = HashMap::new();
        self.fresh_vids(&Ty::Fn(header.clone()), &mut memo)
    }

    /// Normalize `t`, then replace any still-unbound vid with a fresh memoized one. Bound vids
    /// resolve to their concrete types and stay put; only the open slots get fresh handles.
    fn fresh_vids(&mut self, t: &Ty, memo: &mut HashMap<Vid, Vid>) -> Ty {
        match t.clone().normalized(self) {
            Ty::Vid(v) => Ty::Vid(*memo.entry(v).or_insert_with(|| self.vid())),
            Ty::Array(inner) => Ty::Array(Box::new(self.fresh_vids(&inner, memo))),
            Ty::Dict(inner) => Ty::Dict(Box::new(self.fresh_vids(&inner, memo))),
            Ty::Option(inner) => Ty::Option(Box::new(self.fresh_vids(&inner, memo))),
            Ty::Result(inner) => Ty::Result(Box::new(self.fresh_vids(&inner, memo))),
            Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(|t| self.fresh_vids(t, memo)).collect()),
            Ty::Fn(h) => {
                let parameters = h
                    .parameters
                    .iter()
                    .map(|p| {
                        FnParam::new(p.name.clone(), self.fresh_vids(&p.ty, memo), p.has_default)
                    })
                    .collect();
                let return_ty = self.fresh_vids(&h.return_ty, memo);
                Ty::Fn(FnHeader::new(parameters, return_ty, h.is_method))
            }
            other => other,
        }
    }

    fn materialize_slot(&mut self, t: &Option<Ty>, memo: &mut HashMap<u32, Vid>) -> Ty {
        match t {
            Some(t) => self.substitute_anons(t, memo),
            None => Ty::Vid(self.vid()),
        }
    }

    pub(crate) fn substitute_anons(&mut self, t: &Ty, memo: &mut HashMap<u32, Vid>) -> Ty {
        match t {
            Ty::Anon(n) => Ty::Vid(*memo.entry(*n).or_insert_with(|| self.vid())),
            Ty::Array(v) => Ty::Array(Box::new(self.substitute_anons(v, memo))),
            Ty::Dict(v) => Ty::Dict(Box::new(self.substitute_anons(v, memo))),
            Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(|t| self.substitute_anons(t, memo)).collect()),
            Ty::Option(inner) => Ty::Option(Box::new(self.substitute_anons(inner, memo))),
            Ty::Result(inner) => Ty::Result(Box::new(self.substitute_anons(inner, memo))),
            Ty::Fn(h) => Ty::Fn(FnHeader::new(
                h.parameters
                    .iter()
                    .map(|p| {
                        FnParam::new(
                            p.name.clone(),
                            self.substitute_anons(&p.ty, memo),
                            p.has_default,
                        )
                    })
                    .collect(),
                self.substitute_anons(&h.return_ty, memo),
                h.is_method,
            )),
            Ty::Unit
            | Ty::Never
            | Ty::Null
            | Ty::Bool
            | Ty::Int
            | Ty::Float
            | Ty::Str
            | Ty::Vid(_)
            | Ty::Adt(_)
            | Ty::Pacts(_)
            | Ty::Identity(_) => t.clone(),
        }
    }

    /// Registers a native module (e.g. `std::fs`) so that `use` paths and `module::function` calls
    pub fn ensure_module_path(&mut self, path: &[String]) -> AdtId {
        let location = Location::default();
        let mut parent: Option<AdtId> = None;
        let mut leaf: Option<AdtId> = None;
        for segment in path.iter() {
            let existing: Option<AdtId> = match &parent {
                Some(parent_adt) => self.adts[parent_adt]
                    .as_struct()
                    .fields
                    .get(segment)
                    .and_then(|field| field.ty.as_adt().cloned()),
                None => self.library.get(segment).cloned(),
            };

            let adt_ref = if let Some(existing) = existing {
                existing
            } else {
                let mut adt = Adt::new_struct(format!("<module:{segment}>"));
                adt.flags |= AdtFlags::IS_MODULE;
                let id = self.push_adt(adt);
                if let Some(parent_adt) = &parent {
                    let seg_ident = Ident::synthetic(segment.clone());
                    let dec = self.dec_id(&seg_ident, Ty::Adt(id), DecKind::Adt(id), Vis::Public);
                    self.adts[parent_adt].as_struct_mut().insert(
                        segment.clone(),
                        Field {
                            ty: Ty::Adt(id),
                            constant: true,
                            declaration_location: location,
                            dec,
                        },
                    );
                } else {
                    self.library.insert(segment.clone(), id);
                }
                id
            };

            parent = Some(adt_ref);
            leaf = Some(adt_ref);
        }
        leaf.expect("module path must have at least one segment")
    }

    // adt ids arrive pre-allocated from the library's registry, so `self.adts.len()` must
    // equal `api_adt.adt_id.index()` -- library's allocator starts at the builtin count.
    fn install_user_adt(&mut self, api_adt: &ApiAdt) {
        debug_assert_eq!(self.adts.len(), api_adt.adt_id.index());

        let mut variants = IndexMap::new();
        for v in &api_adt.variants {
            let variant = self.build_variant(&v.fields);
            let key = match api_adt.kind {
                ApiAdtKind::Enum => v.name.clone(),
                ApiAdtKind::Struct => Adt::STRUCT_VARIANT_NAME.to_string(),
            };
            variants.insert(key, variant);
        }
        let umbrella = Adt {
            name: api_adt.name.clone(),
            variants,
            impls: HashMap::new(),
            native_overloads: HashMap::new(),
            flags: match api_adt.kind {
                ApiAdtKind::Enum => AdtFlags::IS_ENUM,
                ApiAdtKind::Struct => AdtFlags::empty(),
            },
        };
        let pushed = self.push_adt(umbrella);
        debug_assert_eq!(pushed, api_adt.adt_id);

        if matches!(api_adt.kind, ApiAdtKind::Enum) {
            for v in &api_adt.variants {
                let qualified = format!("{}::{}", api_adt.name, v.name);
                let layout_adt = {
                    let variant = &self.adts[api_adt.adt_id].variants[&v.name];
                    Adt::new_variant_layout(qualified.clone(), variant)
                };
                let layout_pushed = self.push_adt(layout_adt);
                debug_assert_eq!(layout_pushed, v.layout_id);

                let variant_ident = Ident::synthetic(qualified);
                let dec = self.dec_id(
                    &variant_ident,
                    Ty::Adt(api_adt.adt_id),
                    DecKind::Variant {
                        parent: api_adt.adt_id,
                        layout: v.layout_id,
                    },
                    Vis::Public,
                );
                let variant_ref = &mut self.adts[api_adt.adt_id].variants[&v.name];
                variant_ref.set_layout(v.layout_id);
                variant_ref.set_dec(dec);
            }
        }

        if api_adt.module.is_empty() {
            let ident = Ident::synthetic(api_adt.name.clone());
            let ty = Ty::Adt(api_adt.adt_id);
            let dec_id = self.dec_id(&ident, ty, DecKind::Adt(api_adt.adt_id), Vis::Public);
            self.ribs.module_mut().insert(ident, dec_id);
            self.library.insert(api_adt.name.clone(), api_adt.adt_id);
        }
    }

    fn build_variant(&mut self, fields: &ApiVariantFields) -> Variant {
        match fields {
            ApiVariantFields::Unit => Variant::Tuple(TupleVariant {
                members: vec![],
                ..Default::default()
            }),
            ApiVariantFields::Tuple(members) => Variant::Tuple(TupleVariant {
                members: members.clone(),
                ..Default::default()
            }),
            ApiVariantFields::Named(named) => {
                let mut fields_map = IndexMap::new();
                for (name, ty) in named {
                    let ident = Ident::synthetic(name.clone());
                    let dec = self.dec_id(&ident, ty.clone(), DecKind::Local, Vis::Public);
                    fields_map.insert(
                        name.clone(),
                        Field::new(ty.clone(), Location::default(), dec),
                    );
                }
                Variant::Struct(StructVariant {
                    fields: Fields(fields_map),
                    ..Default::default()
                })
            }
        }
    }

    pub fn install_library<C>(&mut self, lib: &Library<C>) {
        for adt in lib.adts() {
            self.install_user_adt(adt);
        }
        for adt in lib.adts() {
            if adt.module.is_empty() {
                continue;
            }
            let leaf = self.ensure_module_path(&adt.module);
            let ident = Ident::synthetic(adt.name.clone());
            let dec = self.dec_id(
                &ident,
                Ty::Adt(adt.adt_id),
                DecKind::Adt(adt.adt_id),
                Vis::Public,
            );
            self.adts[leaf].as_struct_mut().insert(
                adt.name.clone(),
                Field {
                    ty: Ty::Adt(adt.adt_id),
                    constant: true,
                    declaration_location: Default::default(),
                    dec,
                },
            );
        }
        for (id, entry) in lib.natives() {
            match entry {
                // top-level native fn
                ApiEntry::Function(f) if f.module.is_empty() => {
                    self.declare_native_fn(
                        f.name.clone(),
                        f.parameters.clone(),
                        f.return_ty.clone(),
                        id,
                    );
                }
                // module-nested native fn
                ApiEntry::Function(f) => {
                    let leaf = self.ensure_module_path(&f.module);
                    let sig = NativeFnSig {
                        params: f.parameters.clone(),
                        return_ty: f.return_ty.clone(),
                        recv: None,
                    };
                    let ident = Ident::synthetic(f.name.clone());
                    let ty = self.instantiate_native(&sig, None).expect("no recv");
                    let dec_id = self.dec_id(
                        &ident,
                        ty.clone(),
                        DecKind::Item { defaults: vec![] },
                        Vis::Public,
                    );
                    self.dec_to_native.insert(dec_id, NativeBinding { id, sig });
                    self.adts[leaf].as_struct_mut().insert(
                        f.name.clone(),
                        Field {
                            ty,
                            constant: true,
                            declaration_location: Default::default(),
                            dec: dec_id,
                        },
                    );
                }
                // native method or associated fn on a built-in
                ApiEntry::Method(m) => {
                    let adt = match &m.recv_ty {
                        Ty::Adt(id) => *id,
                        other => self.builtin_adt(other).unwrap_or_else(|| {
                            panic!("native method receiver must be a builtin Ty or registered adt; got {other:?}")
                        }),
                    };
                    // recv pattern carries the full receiver shape (e.g. `Ty::Array(Anon(0))`)
                    // so `instantiate_native` can unify it against the actual receiver per call.
                    // primitives like `Ty::Int` have nothing to unify, but it's harmless to pass.
                    let sig = NativeFnSig {
                        params: m.parameters.clone(),
                        return_ty: m.return_ty.clone(),
                        recv: Some(m.recv_ty.clone()),
                    };
                    let ident = Ident::synthetic(m.name.clone());
                    let ty = self.instantiate_native(&sig, None).expect("no recv check");
                    let dec_id = self.dec_id(
                        &ident,
                        ty.clone(),
                        DecKind::Item { defaults: vec![] },
                        Vis::Public,
                    );
                    self.dec_to_native.insert(dec_id, NativeBinding { id, sig });
                    let field = Field {
                        ty,
                        constant: true,
                        declaration_location: Default::default(),
                        dec: dec_id,
                    };
                    // first registration for this name lands in `impls`; subsequent same-name
                    // natives are overloads (e.g. `max_int` + `max_float` both as `max`) and
                    // get dispatched by recv-pattern unification at the call site.
                    if self.adts[adt].impls.contains_key(&m.name) {
                        self.adts[adt]
                            .native_overloads
                            .entry(m.name.clone())
                            .or_default()
                            .push(field);
                    } else {
                        self.adts[adt].impls.insert(m.name.clone(), field);
                    }
                }
                ApiEntry::Constant(c) => {
                    let lit = match &c.value {
                        RawLiteral::Null => Literal::Null,
                        RawLiteral::Bool(true) => Literal::True,
                        RawLiteral::Bool(false) => Literal::False,
                        RawLiteral::Int(i) => Literal::Int(*i),
                        RawLiteral::Float(f) => Literal::Float(*f),
                        RawLiteral::Str(s) => Literal::String(s.clone()),
                    };
                    let ident = Ident::synthetic(c.name.clone());
                    let dec_id = self.dec_id(
                        &ident,
                        c.ty.clone(),
                        DecKind::Constant(Some(lit)),
                        Vis::Public,
                    );
                    // an assoc constant (`Player::MAX_HEALTH`) lands in the adt's impls,
                    // same slot a lang `impl` const hoists into
                    if let Some(recv) = &c.recv_ty {
                        let adt = match recv {
                            Ty::Adt(id) => *id,
                            other => self.builtin_adt(other).unwrap_or_else(|| {
                                panic!("assoc constant receiver must be a builtin Ty or registered adt; got {other:?}")
                            }),
                        };
                        self.adts[adt].impls.insert(
                            c.name.clone(),
                            Field {
                                ty: c.ty.clone(),
                                constant: true,
                                declaration_location: Default::default(),
                                dec: dec_id,
                            },
                        );
                    // empty module = prelude: bind the name directly, same as a top-level
                    // native fn (ensure_module_path has no leaf for an empty path)
                    } else if c.module.is_empty() {
                        self.ribs.module_mut().insert(ident, dec_id);
                    } else {
                        let leaf = self.ensure_module_path(&c.module);
                        self.adts[leaf].as_struct_mut().insert(
                            c.name.clone(),
                            Field {
                                ty: c.ty.clone(),
                                constant: true,
                                declaration_location: Default::default(),
                                dec: dec_id,
                            },
                        );
                    }
                }
            }
        }
    }

    pub(crate) fn register_sub(
        &mut self,
        vid: Vid,
        ty: Ty,
    ) -> std::result::Result<(), UnificationError> {
        // normalize first: a BOUND vid left raw would hit unify's vid arm below, blindly rebind,
        // and re-unify its old binding with the found/expected roles reversed (the
        // cross-module-const inverted-mismatch bug). normalized, the conflict unify below compares
        // concrete types with the caller's roles intact.
        let mut ty = ty.normalized(&*self);

        // trivial self-substitution `V := Vid(V)` is a no-op -- keep any existing sub intact.
        // shows up when an expression's solve returns its own expr_vid (e.g. `d["x"]` whose value
        // type is still an unresolved vid), and Query then folds that result back through
        // register_sub against the same vid.
        if let Ty::Vid(v) = &ty
            && *v == vid
        {
            return Ok(());
        }

        let ty = if let Some(mut previous_ty) = self.subs[vid].take() {
            if let Err(e) =
                Unification::unify(&mut ty, &mut previous_ty, self).and_then(|v| v.commit(self))
            {
                self.subs[vid] = Some(previous_ty);
                return Err(e);
            }
            // `ty` and `previous_ty` now denote the same type under the substitutions just
            // committed; store the resolved form so the binding is the most concrete view rather
            // than whichever one happened to be less inferred.
            previous_ty.normalized(&*self)
        } else {
            ty
        };

        let ty = if let Some(impl_target) = self.impl_target() {
            ty.filter_adt(impl_target)
        } else {
            ty.clone()
        };

        if ty.occurs(vid, self) {
            return Err(UnificationError::recursive(vid, &ty));
        }

        #[cfg(feature = "logging")]
        println!("{}", crate::utils::Printer::substitution(&vid, &ty));
        self.subs[vid] = Some(ty);
        Ok(())
    }
}

// some id utils
impl Solver {
    pub(crate) fn vid(&mut self) -> Vid {
        let vid = self.vids.push(());
        self.subs.push(None);
        vid
    }

    pub(crate) fn node_vid(&mut self, node_id: NodeId) -> Vid {
        if let Some(vid) = self.node_to_vid.get(&node_id) {
            return *vid;
        }
        let vid = self.vid();
        self.node_to_vid.insert(node_id, vid);
        vid
    }

    /// This exists as a shim to provide information to the IR that it would otherwise be able to
    /// find. Take the given example:
    ///
    /// ```
    /// enum Foo {
    ///     Bar,
    /// }
    /// let a: Foo = Foo::Bar;
    /// ```
    ///
    /// The type of a is `Foo`, but to the IR that type is meaningless, as there's no way to
    /// construct `Foo` by itself. Instead, we mark specifically this expression as an Adt built
    /// directly off of the variant so that when the IR inspects this expression, it will know what
    /// it actually should build.
    ///
    /// Bit weird but works.
    pub(crate) fn shadow_expr_ty(
        &mut self,
        node_id: NodeId,
        ty: Ty,
        location: Location,
    ) -> Result<()> {
        let vid = self.node_vid(node_id);
        self.register_sub(vid, ty)
            .map_err(|e| e.into_type_mismatch(self, location))?;
        self.touch_node(node_id);
        Ok(())
    }

    pub(crate) fn sub(&self, vid: Vid) -> Option<&Ty> {
        self.subs[vid].as_ref()
    }

    pub(crate) fn dec_id(&mut self, ident: &Ident, ty: Ty, kind: DecKind, vis: Vis) -> DecId {
        // the binding's type is just a handle into `subs`. if we were handed a bare vid, that's
        // already the handle; otherwise mint one and point it at the type.
        let vid = match ty {
            Ty::Vid(v) => v,
            ty => {
                let v = self.vid();
                self.subs[v] = Some(ty);
                v
            }
        };
        let module = self.ribs.current_module();
        self.decs.push(Dec {
            name: ident.lexeme.clone(),
            location: ident.location,
            kind,
            vid,
            vis,
            module,
        })
    }
}

impl Solver {
    pub(crate) fn solve_pat(&mut self, pat: &Pat, ty: Ty) -> Result<()> {
        // normalize so an unresolved Vid reveals its concrete shape (e.g. tuple destruction where
        // the rhs's type was unified only after we got here).
        let ty = ty.normalized(self);
        match (pat.kind(), ty) {
            (PatKind::Ident(ident), ty) => self.declare(ident, pat.id(), ty).map(|_| ()),
            (PatKind::Tuple(idents), Ty::Tuple(tys)) => match idents.len().cmp(&tys.len()) {
                std::cmp::Ordering::Less => Err(MissingTupleMembers {
                    src: self.src(pat.location()),
                    at: pat.location().into(),
                })?,
                std::cmp::Ordering::Greater => Err(ExtraTupleMembers {
                    src: self.src(pat.location()),
                    at: pat.location().into(),
                })?,
                std::cmp::Ordering::Equal => idents
                    .iter()
                    .zip(tys.iter())
                    .try_for_each(|(pat, ty)| self.solve_pat(pat, ty.clone())),
            },
            (a, b) => Err(InvalidPattern {
                src: self.src(pat.location()),
                at: pat.location().into(),
                pattern: a.to_string(),
                ty: b.to_string(),
            })?,
        }
    }

    /// The set of names a pattern binds. Every alternative of an or-pattern must bind this same
    /// set (same as Rust's E0408). That, plus the shared dec the alternatives unify through, is
    /// what rejects `Foo::Bar(n) | Foo::Buzz(n)` when the payloads differ in type
    fn bound_names(pat: &Pat) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut stack = vec![pat];
        while let Some(pat) = stack.pop() {
            match pat.kind() {
                PatKind::Ident(ident) => {
                    out.insert(ident.lexeme.clone());
                }
                PatKind::Tuple(pats) | PatKind::TupleVariant(_, pats) | PatKind::Or(pats) => {
                    stack.extend(pats);
                }
                PatKind::Struct(_, fields) => stack.extend(fields.values()),
                PatKind::NullBind(inner) => stack.push(inner),
                PatKind::Variant(_) | PatKind::Literal(_) => {}
            }
        }
        out
    }

    /// Solves a match-arm pattern against the scrutinee type. Binds any names the pattern
    /// introduces into the current scope. Unifies literal/variant patterns with the scrutinee
    /// type.
    ///
    /// `reuse` is set while solving the non-first alternatives of an or-pattern: rather than mint a
    /// fresh dec, an ident rebinds to the one the first alternative already declared (looked up in
    /// this arm's own rib), so all alternatives share a single binding and its type. That sharing
    /// is what makes the payload types of `Foo::Bar(n) | Foo::Fizz(n)` unify.
    pub(crate) fn solve_match_pat(&mut self, pat: &Pat, ty: Ty, reuse: bool) -> Result<()> {
        let pat_src = self.src(pat.location());
        let bad = |ty: &Ty| InvalidPattern {
            src: pat_src.clone(),
            at: pat.location().into(),
            pattern: pat.to_string(),
            ty: ty.to_string(),
        };

        let unify = |solver: &mut Solver, mut a: Ty, mut b: Ty, loc: Location| -> Result<()> {
            a.fulfill_ty(&mut b, solver)
                .map_err(|e| e.into_type_mismatch(solver, loc))
        };

        match pat.kind() {
            PatKind::Ident(ident) if reuse => {
                // the or-pattern already checked every alternative binds the same names, so the
                // first alternative's dec is guaranteed present in this arm's rib.
                let dec = self
                    .ribs
                    .resolve_current_only(ident)
                    .expect("first or-alternative declared this binding");
                self.node_decs.insert(pat.id(), dec);
                unify(self, Ty::Vid(self.decs[dec].vid), ty, pat.location())
            }
            PatKind::Ident(ident) => self.declare(ident, pat.id(), ty).map(|_| ()),
            PatKind::Literal(lit) => unify(
                self,
                ty,
                match lit {
                    parse::Literal::True | parse::Literal::False => Ty::Bool,
                    parse::Literal::Null => Ty::Null,
                    parse::Literal::Unit => Ty::Unit,
                    parse::Literal::String(_) => Ty::Str,
                    parse::Literal::Int(_) | parse::Literal::Hex(_) => Ty::Int,
                    parse::Literal::Float(_) => Ty::Float,
                    _ => Ty::Vid(crate::components::Vid::UNKNOWN),
                },
                pat.location(),
            ),
            PatKind::Tuple(pats) => {
                let normalized = ty.normalized(self);
                let Ty::Tuple(tys) = normalized else {
                    Err(bad(&normalized))?
                };
                if pats.len() != tys.len() {
                    Err(bad(&Ty::Tuple(tys.clone())))?
                }
                for (p, t) in pats.iter().zip(tys) {
                    self.solve_match_pat(p, t, reuse)?;
                }
                Ok(())
            }
            PatKind::NullBind(inner_pat) => {
                let normalized = ty.normalized(self);
                let inner = match &normalized {
                    Ty::Option(inner) | Ty::Result(inner) => inner.as_ref().clone(),
                    _ => Err(bad(&normalized))?,
                };
                // the IR reads this node's ty to pick the success test (null vs raised)
                self.shadow_expr_ty(pat.id(), normalized, pat.location())?;
                self.solve_match_pat(inner_pat, inner, reuse)
            }
            PatKind::Or(alts) => {
                let Some((first, rest)) = alts.split_first() else {
                    return Ok(());
                };
                // the first alternative declares the canonical bindings (or, if this or-pattern is
                // itself nested in a reuse context, rebinds the enclosing ones); the rest reuse.
                self.solve_match_pat(first, ty.clone(), reuse)?;
                let expected = Self::bound_names(first);
                for alt in rest {
                    if let Some(name) = expected
                        .symmetric_difference(&Self::bound_names(alt))
                        .next()
                    {
                        Err(crate::errors::InconsistentOrBindings {
                            src: self.src(pat.location()),
                            at: pat.location().into(),
                            name: name.clone(),
                        })?
                    }
                    self.solve_match_pat(alt, ty.clone(), true)?;
                }
                Ok(())
            }
            PatKind::Variant(_) | PatKind::TupleVariant(_, _) | PatKind::Struct(_, _) => {
                let path = match pat.kind() {
                    PatKind::Variant(p) | PatKind::TupleVariant(p, _) | PatKind::Struct(p, _) => p,
                    _ => unreachable!(),
                };
                let (adt, variant_name) = match path.kind() {
                    ExprKind::Ident(ident) => {
                        let ty = self.resolve_name(ident, ident.location())?;
                        let Ty::Adt(adt) = ty else { Err(bad(&ty))? };
                        (adt, None)
                    }
                    ExprKind::Access(Access::DoubleColon { left, right }) => {
                        let adt = match left.query(self)? {
                            Ty::Adt(adt) | Ty::Identity(adt) => adt,
                            lty => Err(bad(&lty))?,
                        };
                        // module::struct -- treat as the inner struct adt
                        if self.adts[adt].flags.contains(AdtFlags::IS_MODULE) {
                            let field = self.adts[adt]
                                .as_struct()
                                .fields
                                .get(&right.lexeme)
                                .cloned()
                                .ok_or_else(|| FieldNotFound {
                                    src: self.src(right.location),
                                    at: right.location.into(),
                                    field_name: right.lexeme.clone(),
                                })?;
                            let Ty::Adt(inner) = field.ty else {
                                Err(bad(&field.ty))?
                            };
                            (inner, None)
                        } else {
                            // enum variant
                            if !self.adts[adt].variants.contains_key(&right.lexeme) {
                                Err(crate::errors::VariantNotFound {
                                    src: self.src(right.location),
                                    at: right.location.into(),
                                    name: right.lexeme.clone(),
                                })?
                            }
                            (adt, Some(right.lexeme.clone()))
                        }
                    }
                    _ => Err(InvalidPattern {
                        src: self.src(path.location()),
                        at: path.location().into(),
                        pattern: path.to_string(),
                        ty: "<path>".to_string(),
                    })?,
                };

                unify(self, ty, Ty::Adt(adt), pat.location())?;
                let variant_key = variant_name
                    .unwrap_or_else(|| self.adts[adt].variants.keys().next().unwrap().clone());
                let variant = self.adts[adt]
                    .variants
                    .get(&variant_key)
                    .ok_or_else(|| bad(&Ty::Adt(adt)))?
                    .clone();
                // shadow the path with the layout adt id (per-variant for enums, the parent for
                // non-enum structs) so IR can read it to emit IsInstance / GetField.
                let layout = variant.layout().unwrap_or(adt);
                self.shadow_expr_ty(path.id(), Ty::Adt(layout), path.location())?;
                match (pat.kind(), variant) {
                    (PatKind::Variant(_), _) => Ok(()),
                    (PatKind::TupleVariant(_, sub_pats), Variant::Tuple(tv)) => {
                        if tv.members.len() != sub_pats.len() {
                            Err(bad(&Ty::Adt(adt)))?
                        }
                        for (sp, mty) in sub_pats.iter().zip(tv.members) {
                            self.solve_match_pat(sp, mty, reuse)?;
                        }
                        Ok(())
                    }
                    (PatKind::Struct(_, field_pats), Variant::Struct(sv)) => {
                        for (name, sub) in field_pats {
                            let fty =
                                sv.fields.get(name).map(|f| f.ty.clone()).ok_or_else(|| {
                                    FieldNotFound {
                                        src: self.src(sub.location()),
                                        at: sub.location().into(),
                                        field_name: name.clone(),
                                    }
                                })?;
                            self.solve_match_pat(sub, fty, reuse)?;
                        }
                        Ok(())
                    }
                    _ => Err(bad(&Ty::Adt(adt)))?,
                }
            }
        }
    }
}

// Internal impls
impl Solver {
    pub(crate) fn visit_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        #[cfg(feature = "logging")]
        println!("{}", crate::utils::Printer::stmt(stmt));
        match stmt.kind() {
            StmtKind::Assignment(Assignment { left, right, op }) => {
                fn find_root(left: &Expr) -> std::result::Result<&Ident, Location> {
                    match left.kind() {
                        ExprKind::Ident(ident) => Ok(ident),
                        ExprKind::Access(Access::Dot { left, .. })
                        | ExprKind::Access(Access::Square { left, .. }) => find_root(left),
                        _ => Err(left.location()),
                    }
                }

                let root = find_root(left).map_err(|loc| InvalidAssignTarget {
                    src: self.src(loc),
                    at: loc.into(),
                })?;
                let dec_id = self.ribs.resolve(root).ok_or_else(|| NotFound {
                    src: self.src(root.location),
                    at: root.location.into(),
                    name: root.lexeme.clone(),
                })?;

                match self.decs[dec_id].kind {
                    DecKind::Constant(_) => Err(AssignToConst {
                        src: self.src(left.location()),
                        at: left.location().into(),
                        name: root.lexeme.clone(),
                    })?,
                    // only `x =` itself matters, if x was something like an adt `x.y =` would be
                    // fine
                    DecKind::LoopVar if matches!(left.kind(), ExprKind::Ident(_)) => {
                        Err(AssignToLoopVar {
                            src: self.src(left.location()),
                            at: left.location().into(),
                            name: root.lexeme.clone(),
                        })?
                    }
                    _ => {}
                }

                if let ExprKind::Access(Access::Square { left: receiver, .. }) = left.kind() {
                    let receiver = receiver.query(self)?.normalized(self);
                    if receiver == Ty::Str
                        || matches!(&receiver, Ty::Option(inner) if **inner == Ty::Str)
                    {
                        Err(AssignToStringIndex {
                            src: self.src(left.location()),
                            at: left.location().into(),
                        })?
                    }
                }

                // `??=` mirrors read-form `??`: the target must be an option and the rhs
                // must fulfill its inner type (Coalescence::solve enforces the same).
                if *op == parse::AssignmentOp::NullCoalescenceEqual {
                    let lhs = left.query(self)?.normalized(self);
                    match lhs {
                        Ty::Option(inner) => right.fulfill_ty(*inner, self)?,
                        other => Err(crate::errors::CoalesceNonOptional {
                            src: self.src(left.location()),
                            at: left.location().into(),
                            ty: other.to_string(),
                        })?,
                    }
                } else {
                    right.fulfill_ty(left.query(self)?, self)?
                }
            }
            StmtKind::Let(Let {
                left,
                annotation,
                right,
                else_branch,
            }) => {
                let vid = self.node_vid(right.id());
                let mut ty = Ty::Vid(vid);

                if let Some(annotation) = &annotation {
                    ty.fulfill_ty(&mut Ty::from_annotation(annotation.clone(), self)?, self)
                        .map_err(|e| e.into_type_mismatch(self, left.location()))?;
                };

                right.fulfill_ty(ty.clone(), self)?;

                if annotation.is_none() && ty.clone().normalized(self) == Ty::Null {
                    Err(BareNullBinding {
                        src: self.src(right.location()),
                        at: right.location().into(),
                    })?;
                }

                // a bare pact name has a Ty (so accesses like `P::CONST` can consume it) but
                // no value form -- binding one would ICE at codegen. pact-ANNOTATED lets with
                // a concrete rhs are fine, so key off the rhs expr being the pact ident itself.
                if matches!(right.kind(), ExprKind::Ident(_))
                    && let Some(&dec) = self.node_decs.get(&right.id())
                    && matches!(self.decs[dec].kind, DecKind::Pact(_))
                {
                    Err(crate::errors::PactIsNotAValue {
                        src: self.src(right.location()),
                        at: right.location().into(),
                    })?;
                }

                match else_branch {
                    // no `else` -> the pattern must be irrefutable (ident / tuple destructure)
                    None => self.solve_pat(left, ty)?,
                    // let-else: solve the `else` BEFORE binding (its names must not be in scope
                    // there) and isolate its flow so the diverging `else` doesn't make the rest
                    // of the block read as unreachable. then bind refutably.
                    Some(else_branch) => {
                        self.control_flow.enter();
                        let else_ty = else_branch.query(self)?;
                        self.control_flow.exit();
                        if else_ty.normalized(self) != Ty::Never {
                            Err(crate::errors::LetElseMustDiverge {
                                src: self.src(else_branch.location()),
                                at: else_branch.location().into(),
                            })?;
                        }
                        self.solve_match_pat(left, ty, false)?;
                    }
                }
            }
            StmtKind::Expr(expr) => expr.query(self).map(|_| ())?,
            StmtKind::Module(_) => {
                // module decls are handled in the pre-pass of `solve`.
            }
            StmtKind::Item(item) => match item.kind() {
                // block-scoped consts get solved here, top-level consts are handled by the
                // hoist_constants/solve_consts fixpoint
                ItemKind::Const(con) => self.solve_const(con, item.id()).map(|_| ())?,
                // re-runs `process_use` so the imports land in the body-solve phase's fresh
                // import rib (hoist_uses pushed them, but its rib was popped at phase end).
                ItemKind::Use(us) => self.process_use(us, item.location())?,
                // all we need to do at this point is process the fn defaults
                ItemKind::Pact(pact) => {
                    let Some(pid) = self
                        .resolve_name(&pact.name, pact.name.location)?
                        .as_single_pact()
                    else {
                        unreachable!("a pact name always resolves to a single pact");
                    };
                    let prev_self = self.pact_self.replace(pid);
                    for item in pact.items.iter() {
                        let PactItem::Fn {
                            default: Some(body),
                            name: _,
                            parameters,
                            return_type,
                        } = item
                        else {
                            continue;
                        };

                        self.ribs.push_function();
                        for binding in parameters {
                            let param = self.bind_parameter(binding)?;
                            self.declare(
                                binding.left.as_ident().unwrap(),
                                binding.left.id(),
                                param.ty,
                            )?;
                        }
                        let expected = return_type
                            .clone()
                            .map_or(Ok(Ty::Unit), |a| Ty::from_annotation(a, self))?;
                        self.fn_stack.push(FnRun {
                            expected_ty: expected.clone(),
                        });
                        self.control_flow.enter();
                        let body_ty = body.query(self)?;
                        self.fn_stack.pop();
                        self.control_flow.exit();
                        if body_ty != Ty::Unit {
                            body.fulfill_ty(expected, self)?;
                        }
                        self.ribs.pop();
                    }
                    self.pact_self = prev_self;
                }
                // each of these needs its inner Solve impl run during the body pass -- that's
                // where field-annotation unification (Struct/Enum), body type-checking
                // (Function), and method-body solving (Impl) happens. mirrors `Query for Expr`'s
                // touch/solve/register_sub dance for memoization. listed explicitly so new
                // ItemKinds force a compile error rather than silently panicking.
                ItemKind::Function(_)
                | ItemKind::Struct(_)
                | ItemKind::Enum(_)
                | ItemKind::Impl(_) => {
                    if self.touch_node(item.id()) {
                        let ty = match item.kind() {
                            ItemKind::Function(f) => f.solve(item.id(), item.location(), self)?,
                            ItemKind::Struct(s) => s.solve(item.id(), item.location(), self)?,
                            ItemKind::Enum(e) => e.solve(item.id(), item.location(), self)?,
                            ItemKind::Impl(i) => i.solve(item.id(), item.location(), self)?,
                            // narrowed by the outer arm -- inner exhaustiveness can't see it
                            _ => unreachable!("outer match restricts to Function/Struct/Enum/Impl"),
                        };
                        let vid = self.node_vid(item.id());
                        self.register_sub(vid, ty)
                            .map_err(|e| e.into_type_mismatch(self, item.location()))?;
                    }
                }
            },
        }
        Ok(())
    }

    pub fn resolve_name(&mut self, ident: &Ident, read_location: Location) -> Result<Ty> {
        if ident.is_identity() {
            // inside a pact, both `self` and `Self` resolve to the pact itself (any impl type)
            if let Some(pid) = self.pact_self {
                return Ok(Ty::pacts(vec![pid]));
            }
            return self.impl_target().map(Ty::Adt).ok_or_else(|| {
                SelfOutOfContext {
                    src: self.src(read_location),
                    at: read_location.into(),
                }
                .into()
            });
        }
        match self.ribs.resolve(ident) {
            Some(dec_id) => {
                self.check_vis(dec_id, read_location)?;
                // natives carry a fresh `Ty::Fn` per use so each call gets its own type vars;
                // without this, two calls with different types unify and the second fails.
                if let Some(binding) = self.dec_to_native.get(&dec_id) {
                    let sig = NativeFnSig {
                        params: binding.sig.params.clone(),
                        return_ty: binding.sig.return_ty.clone(),
                        recv: binding.sig.recv.clone(),
                    };
                    return self.instantiate_native(&sig, None);
                }
                Ok(Ty::Vid(self.decs[dec_id].vid).normalized(self))
            }
            None if self.library.contains_key(&ident.lexeme) => {
                Ok(Ty::Adt(self.library[&ident.lexeme]))
            }
            None => {
                if let Some((pid, _)) = self.pacts.iter().find(|(_, p)| p.name == ident.lexeme) {
                    return Ok(Ty::pacts(vec![pid]));
                }
                Err(NotFound {
                    src: self.src(read_location),
                    at: read_location.into(),
                    name: ident.lexeme.clone(),
                }
                .into())
            }
        }
    }

    /// Const-folds an expr using the solver's dec/lit tables to resolve Ident leaves. Returns
    /// the literal iff every leaf either reduces syntactically or is a const-bound ident whose
    /// value is already known. Errors (e.g. divide-by-zero) propagate up as diagnostics --
    /// reduce returns raw location-tagged errors and we materialize them with source here.
    pub(crate) fn reduce_const_expr(&self, expr: &Expr) -> Result<Option<Literal>> {
        crate::utils::reduce(expr, &|e| {
            self.node_decs
                .get(&e.id())
                .and_then(|&dec| self.decs[dec].kind.const_value().cloned())
        })
        .map_err(|e| e.into_diag(self))
    }

    pub(crate) fn is_const_expr(&self, expr: &Expr) -> bool {
        match expr.kind() {
            ExprKind::Ident(_) => self
                .node_decs
                .get(&expr.id())
                .is_some_and(|&dec| self.decs[dec].kind.is_constant()),
            ExprKind::Literal(parse::Literal::Array(elems) | parse::Literal::Tuple(elems)) => {
                elems.iter().all(|e| self.is_const_element(e))
            }
            ExprKind::Literal(_) => true,
            ExprKind::Grouping(g) => self.is_const_expr(&g.inner),
            ExprKind::Unary(u) => self.is_const_expr(&u.right),
            ExprKind::Evaluation(e) => self.is_const_expr(&e.left) && self.is_const_expr(&e.right),
            ExprKind::Equality(e) => self.is_const_expr(&e.left) && self.is_const_expr(&e.right),
            ExprKind::Logical(l) => self.is_const_expr(&l.left) && self.is_const_expr(&l.right),
            _ => false,
        }
    }

    // const array/tuple elements must encode as a bytecode `Constant` (scalars or nested
    // arrays/tuples of scalars). dict/struct consts only exist as use-site allocations
    // (emit_const_dec), so they can't appear inside one -- directly or through a const ref.
    fn is_const_element(&self, expr: &Expr) -> bool {
        match expr.kind() {
            ExprKind::Literal(parse::Literal::Dictionary(_) | parse::Literal::Struct(_)) => false,
            ExprKind::Literal(parse::Literal::Array(es) | parse::Literal::Tuple(es)) => {
                es.iter().all(|e| self.is_const_element(e))
            }
            ExprKind::Ident(_) => {
                self.node_decs
                    .get(&expr.id())
                    .is_some_and(|&dec| match &self.decs[dec].kind {
                        DecKind::Constant(Some(
                            parse::Literal::Dictionary(_) | parse::Literal::Struct(_),
                        )) => false,
                        kind => kind.is_constant(),
                    })
            }
            _ => self.is_const_expr(expr),
        }
    }

    pub(crate) fn impl_target(&self) -> Option<AdtId> {
        self.ribs.impl_target()
    }

    /// Resolves a function/closure parameter's type into an `FnParam` for the `FnHeader`: takes
    /// annotation if present, unifies with the default value's type if present, fixes `self` to
    /// the current impl target if any. Allocates one `Vid` per param and unifies into it
    /// directly via `TyExt::fulfill_ty`, and does not create a `Dec` or touch `node_decs`. The
    /// real per-body binding happens in `Function::solve`, which is the sole writer of the param's
    /// dec; this used to double-bind (throwaway Dec here + real Dec there).
    pub(crate) fn bind_parameter(&mut self, binding: &Binding) -> Result<FnParam> {
        let Binding {
            left,
            right,
            annotation,
        } = binding;
        let ident = left.as_ident().unwrap();
        let vid = self.vid();
        let mut ty = Ty::Vid(vid);

        // `self` inside an impl method gets its type fixed to the impl target -- same effect as
        // writing `self: Self` explicitly. outside an impl, the body-pass declare leaves it as a
        // vid and SelfOutOfContext fires at first use.
        if ident.lexeme == "self"
            && let Some(adt) = self.impl_target()
        {
            let mut target = Ty::Adt(adt);
            ty.fulfill_ty(&mut target, self)
                .map_err(|e| e.into_type_mismatch(self, ident.location))?;
        }

        if let Some(annotation) = annotation.clone() {
            let mut target = Ty::from_annotation(annotation, self)?;
            ty.fulfill_ty(&mut target, self)
                .map_err(|e| e.into_type_mismatch(self, ident.location))?;
        }
        let has_default = if let Some(right) = right {
            let mut rhs_ty = right.query(self)?;
            ty.fulfill_ty(&mut rhs_ty, self)
                .map_err(|e| e.into_type_mismatch(self, ident.location))?;
            true
        } else {
            false
        };

        Ok(FnParam::new(
            Some(ident.lexeme.clone()),
            Ty::Vid(vid).normalized(self),
            has_default,
        ))
    }

    /// Binds a local into the current scope.
    pub(crate) fn declare(&mut self, ident: &Ident, node_id: NodeId, ty: Ty) -> Result<Ty> {
        self.declare_internal(ident, Some(node_id), ty)
    }

    fn declare_import(&mut self, import: ImportBinding) -> Result<Ty> {
        let ident = Ident::synthetic(import.name);
        let dec_id = if let Some(dec) = import.dec {
            dec
        } else {
            let kind = if import.constant {
                DecKind::Constant(None)
            } else {
                DecKind::Item { defaults: vec![] }
            };
            self.dec_id(&ident, import.ty.clone(), kind, Vis::Public)
        };
        self.ribs.import_mut().insert(ident, dec_id);
        Ok(import.ty)
    }

    fn declare_internal(&mut self, ident: &Ident, node_id: Option<NodeId>, ty: Ty) -> Result<Ty> {
        #[cfg(feature = "logging")]
        println!("{}", crate::utils::Printer::declare(ident, &ty));

        let dec_id = self.dec_id(ident, ty.clone(), DecKind::Local, Vis::Public);
        if let Some(node_id) = node_id {
            self.node_decs.insert(node_id, dec_id);
        }
        self.ribs.current_mut().insert(ident.clone(), dec_id);
        Ok(ty)
    }

    /// Binds a hoisted item (fn, type, or `const`) into the root module scope,
    /// so it resolves from anywhere, including across function boundaries.
    pub(crate) fn declare_item(
        &mut self,
        ident: &Ident,
        ty: Ty,
        node_id: Option<NodeId>,
        constant: bool,
        vis: Vis,
    ) -> Result<Ty> {
        #[cfg(feature = "logging")]
        println!("{}", crate::utils::Printer::declare(ident, &ty));

        if constant && let Some(prev_id) = self.ribs.module_lookup(ident) {
            let prev = &self.decs[prev_id];
            if prev.kind.is_constant() {
                let prev_location = prev.location;
                Err(MultipleConstDeclarations {
                    src: self.src(ident.location),
                    name: ident.lexeme.clone(),
                    at: ident.location.into(),
                    original: vec![crate::errors::ConstDefinedHere {
                        src: self.src(prev_location),
                        at: prev_location.into(),
                        name: ident.lexeme.clone(),
                    }],
                })?;
            }
        }

        let kind = if constant {
            DecKind::Constant(None)
        } else {
            DecKind::Item { defaults: vec![] }
        };
        let dec_id = self.dec_id(ident, ty.clone(), kind, vis);
        self.ribs.module_mut().insert(ident.clone(), dec_id);

        if let Some(node_id) = node_id {
            self.node_decs.insert(node_id, dec_id);
        }

        Ok(ty)
    }

    pub(crate) fn run_loop_body(&mut self, body: &Expr) -> Result<LoopRun> {
        self.loop_stack.push(LoopRun {
            collect_ty: None,
            break_ty: None,
        });
        body.fulfill_ty(Ty::Unit, self)?;
        Ok(self
            .loop_stack
            .pop()
            .map(|mut l| {
                l.break_ty = l.break_ty.map(|v| v.normalized(self));
                l.collect_ty = l.collect_ty.map(|v| v.normalized(self));
                l
            })
            .unwrap())
    }

    pub(crate) fn regsiter_loop_control_flow_ty(
        &mut self,
        mut ty: Ty,
        target_option: &mut Option<Ty>,
    ) -> std::result::Result<(), UnificationError> {
        if let Some(break_ty) = target_option
            && let Err(e) = ty.fulfill_ty(break_ty, self)
        {
            ty = Ty::coerce_option(ty, break_ty.clone(), self).ok_or(e)?;
        }

        *target_option = Some(ty);
        Ok(())
    }

    /// Runs `f` against the innermost loop frame. The frame is detached for the duration so `f`
    /// can hold `&mut Session` alongside `&mut LoopRun`.
    pub(crate) fn with_loop_mut<R>(
        &mut self,
        not_in_loop: impl FnOnce() -> Error,
        f: impl FnOnce(&mut Solver, &mut LoopRun) -> Result<R>,
    ) -> Result<R> {
        let mut loop_data = self.loop_stack.pop().ok_or_else(not_in_loop)?;
        let r = f(self, &mut loop_data);
        self.loop_stack.push(loop_data);
        r
    }

    pub(crate) fn touch_node(&mut self, node_id: NodeId) -> bool {
        self.node_visits.insert(node_id)
    }
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct LoopRun {
    pub collect_ty: Option<Ty>,
    pub break_ty: Option<Ty>,
}

pub(crate) struct FnRun {
    pub expected_ty: Ty,
}

// Per-dec `Option<Ty>`-shape signature for native callables. Lets each Ident resolution mint
// fresh vids per call site instead of sharing one set of inference variables. `recv` carries the
// pattern type for the receiver (e.g. `[Anon(0)]` for `Vec<T>`) -- methods use it at dispatch to
// reject receivers whose element type doesn't fit.
pub(crate) struct NativeFnSig {
    pub params: Vec<Option<Ty>>,
    pub return_ty: Option<Ty>,
    pub recv: Option<Ty>,
}

pub(crate) struct NativeBinding {
    pub id: NativeId,
    pub sig: NativeFnSig,
}

/// One module-nested native function, as handed to [`Solver::register_native_module`].
pub struct NativeModuleFn {
    pub name: String,
    pub params: Vec<Option<Ty>>,
    pub return_ty: Option<Ty>,
    pub native_id: NativeId,
}

struct ImportBinding {
    name: String,
    ty: Ty,
    dec: Option<DecId>,
    constant: bool,
}
