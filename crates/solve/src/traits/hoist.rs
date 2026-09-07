use std::collections::HashSet;

use super::Query;
use crate::{
    Result,
    components::{
        Adt, AdtId, DecKind, Field, Fields, FnHeader, StructVariant, TupleVariant, Ty, TyExt,
        Variant, Vis,
    },
    errors::{DuplicateImplDeclaration, DuplicatePactImpl, InvalidImplTarget},
    *,
};
use indexmap::IndexMap;
use parse::{
    components::{Annotation, Binding},
    *,
};
use shared::{Located, Location};

/// Utility passed around by expressions/statements in the declaration pass of the type checker.
pub(crate) struct HoistCtx<'a> {
    solver: &'a mut Solver,
    target: HoistTarget,
    node_id: Option<NodeId>,
    location: Location,
    vis: Vis,
}

impl<'a> HoistCtx<'a> {
    /// Creates a new ctx. `vis` is the visibility of the item being hoisted -- used by `write`
    /// and `write_adt` for the resulting dec.
    pub(crate) fn new(
        solver: &'a mut Solver,
        target: HoistTarget,
        node_id: Option<NodeId>,
        location: Location,
        vis: Vis,
    ) -> Self {
        Self {
            solver,
            target,
            node_id,
            location,
            vis,
        }
    }

    pub(crate) fn write_adt(&mut self, ident: &Ident, adt_id: AdtId) -> Result<()> {
        debug_assert!(
            matches!(self.target, HoistTarget::Scope),
            "adt decls only hoist at scope level"
        );
        let ty = Ty::Adt(adt_id);
        let dec_id = self
            .solver
            .dec_id(ident, ty.clone(), DecKind::Adt(adt_id), self.vis);
        self.solver.ribs.module_mut().insert(ident.clone(), dec_id);
        if let Some(node_id) = self.node_id {
            self.solver.node_decs.insert(node_id, dec_id);
            let vid = self.solver.node_vid(node_id);
            let loc = self.location;
            self.solver
                .register_sub(vid, ty)
                .map_err(|e| e.into_type_mismatch(self.solver, loc))?;
        }
        Ok(())
    }

    /// Completes a hoist by writing it to the needed targets.
    pub(crate) fn write(&mut self, ident: &Ident, mut ty: Ty, constant: bool) -> Result<()> {
        if let HoistTarget::Adt(adt) = &self.target {
            ty = ty.filter_adt(*adt);
        }

        match &self.target {
            HoistTarget::Scope => {
                self.solver
                    .declare_item(ident, ty.clone(), self.node_id, constant, self.vis)?;
            }
            HoistTarget::Adt(adt) => {
                let kind = if constant {
                    DecKind::Constant(None)
                } else {
                    DecKind::Item { defaults: vec![] }
                };

                if let Some(existing) = self.solver.adts[*adt].impls.get(&ident.lexeme) {
                    let prev = &self.solver.decs[existing.dec];
                    Err(DuplicateImplDeclaration {
                        src: self.solver.src(ident.location),
                        at: ident.location.into(),
                        original_at: prev.location.into(),
                        name: ident.lexeme.clone(),
                    })?;
                }

                let dec = self.solver.dec_id(ident, ty.clone(), kind, self.vis);
                self.solver.adts[adt].impls.insert(
                    ident.lexeme.clone(),
                    Field::new_constant(ty.clone(), self.location, dec),
                );
                // def-site mapping: lets the item's own body lower via node_dec(expr.id())
                if let Some(node_id) = self.node_id {
                    self.solver.node_decs.insert(node_id, dec);
                }
            }
        }

        if let Some(node_id) = self.node_id {
            let vid = self.solver.node_vid(node_id);
            let loc = self.location;
            self.solver
                .register_sub(vid, ty)
                .map_err(|e| e.into_type_mismatch(self.solver, loc))?;
        }

        Ok(())
    }
}

/// The target for a given hoist (where it should be written).
#[derive(Debug, Clone)]
pub(crate) enum HoistTarget {
    /// This hoist will get written into the current scope.
    Scope,
    /// This hoist will get insderted into a struct.
    Adt(AdtId),
}

/// Used for expressions/statements that need to define named values in the first pass of the type
/// checker such as structs, traits, impls, and functions.
pub(crate) trait Hoist {
    /// Declares this type into the solver.
    fn hoist(&self, ctx: HoistCtx) -> Result<()>;
}

impl Hoist for parse::item::Enum {
    fn hoist(&self, mut ctx: HoistCtx) -> Result<()> {
        let variants = self
            .members
            .iter()
            .map(|(name, member)| {
                let variant = match member {
                    Member::Tuple(members) => Variant::Tuple(TupleVariant {
                        members: members.iter().map(|_| ctx.solver.vid().into()).collect(),
                        location: Some(name.location),
                        ..Default::default()
                    }),
                    Member::Struct(fields) => Variant::Struct(StructVariant {
                        fields: Fields(
                            fields
                                .iter()
                                .map(|StructField { name, location, .. }| {
                                    let ty: Ty = ctx.solver.vid().into();
                                    let ident = parse::Ident {
                                        lexeme: name.to_string(),
                                        location: *location,
                                    };
                                    let dec = ctx.solver.dec_id(
                                        &ident,
                                        ty.clone(),
                                        DecKind::Local,
                                        Vis::Public,
                                    );
                                    (name.to_string(), Field::new(ty, *location, dec))
                                })
                                .collect(),
                        ),
                        location: Some(name.location),
                        ..Default::default()
                    }),
                };

                (name.to_string(), variant)
            })
            .collect();

        let adt = crate::components::Adt::new_enum(self.head.to_string(), variants);
        let id = ctx.solver.push_adt(adt);

        let variant_names: Vec<_> = ctx.solver.adts[id].variants.keys().cloned().collect();
        for name in variant_names {
            let qualified = format!("{}::{}", self.head, name);

            let layout_adt = {
                let variant = &ctx.solver.adts[id].variants[&name];
                Adt::new_variant_layout(qualified.clone(), variant)
            };
            let layout = ctx.solver.push_adt(layout_adt);

            let variant_ident = parse::Ident::synthetic(qualified);
            let dec = ctx.solver.dec_id(
                &variant_ident,
                Ty::Adt(id),
                DecKind::Variant { parent: id, layout },
                Vis::Private,
            );

            let v = &mut ctx.solver.adts[id].variants[&name];
            v.set_layout(layout);
            v.set_dec(dec);
        }

        ctx.write_adt(&self.head, id)
    }
}

impl Hoist for Struct {
    fn hoist(&self, mut ctx: HoistCtx) -> Result<()> {
        let is_tuple = !self.fields.is_empty()
            && self
                .fields
                .iter()
                .all(|f| matches!(f.name, FieldKey::Int(_)));

        let adt = if is_tuple {
            let members = self
                .fields
                .iter()
                .map(|_| Ty::Vid(ctx.solver.vid()))
                .collect();

            Adt::new_tuple_struct(self.name.to_string(), members)
        } else {
            let mut adt = Adt::new_struct(self.name.to_string());

            self.fields.iter().for_each(
                |StructField {
                     name,
                     location,
                     public,
                     ..
                 }| {
                    let ty = Ty::Vid(ctx.solver.vid());
                    let ident = parse::Ident {
                        lexeme: name.to_string(),
                        location: *location,
                    };
                    let dec = ctx.solver.dec_id(
                        &ident,
                        ty.clone(),
                        DecKind::Local,
                        if *public { Vis::Public } else { Vis::Private },
                    );
                    adt.as_struct_mut()
                        .insert(name.to_string(), Field::new(ty, *location, dec));
                },
            );
            adt
        };

        let id = ctx.solver.push_adt(adt);
        ctx.write_adt(&self.name, id)
    }
}

impl Hoist for Impl {
    fn hoist(&self, ctx: HoistCtx) -> Result<()> {
        // Find our target
        let Ty::Adt(adt) = self.target.query(ctx.solver)? else {
            Err(InvalidImplTarget {
                src: ctx.solver.src(self.target.location()),
                at: self.target.location().into(),
            })?
        };

        // Record pact membership up front (before any body solves) so coercions resolve regardless
        // of source order. Implementing the same pact twice is rejected here; member-name clashes
        // with anything else already on the adt surface as the members land in `impls` below.
        let pid = self.pact.as_ref().and_then(|pact| {
            ctx.solver
                .resolve_name(pact, pact.location)
                .ok()
                .and_then(|t| t.as_single_pact())
        });
        if let Some(pid) = pid
            && !ctx.solver.pact_impls.insert((pid, adt))
        {
            Err(DuplicatePactImpl {
                src: ctx.solver.src(self.target.location()),
                at: self.target.location().into(),
                target: self.target.lexeme.clone(),
                pact: ctx.solver.pacts[pid].name.clone(),
            })?;
        }

        ctx.solver.ribs.push_impl(adt);

        for item in self.items.iter() {
            let vis = if item.public() {
                Vis::Public
            } else {
                Vis::Private
            };
            let hoist_ctx = HoistCtx::new(
                ctx.solver,
                HoistTarget::Adt(adt),
                Some(item.id()),
                ctx.location,
                vis,
            );
            match item.kind() {
                ItemKind::Const(con) => con.hoist(hoist_ctx)?,
                ItemKind::Function(function) => function.hoist(hoist_ctx)?,
                // parser rejects everything else in impl-body position
                _ => unreachable!("parser only allows fn/const in impl bodies"),
            };
        }

        // graft methods the impl omitted but the pact defaults, so they become real callable
        // methods on the adt (and collide like everything else if another pact already provides
        // the name). freshen the header so each impl's `self` slot is independent.
        if let Some(pid) = pid {
            let provided: HashSet<String> = self
                .items
                .iter()
                .filter_map(|i| match i.kind() {
                    ItemKind::Function(f) => Some(f.name.lexeme.clone()),
                    _ => None,
                })
                .collect();
            let omitted: Vec<String> = ctx.solver.pacts[pid]
                .functions
                .iter()
                .filter(|&(name, (_, default))| {
                    default.is_some() && !provided.contains(name.as_str())
                })
                .map(|(name, _)| name.clone())
                .collect();
            for name in omitted {
                if let Some(existing) = ctx.solver.adts[adt].impls.get(&name) {
                    let original_at = ctx.solver.decs[existing.dec].location.into();
                    Err(DuplicateImplDeclaration {
                        src: ctx.solver.src(self.target.location()),
                        at: self.target.location().into(),
                        original_at,
                        name: name.clone(),
                    })?;
                }
                let header = ctx.solver.pacts[pid].functions[&name].0.clone();
                let ty = ctx.solver.instantiate_pact_fn(&header);
                let dec = ctx.solver.pact_default_decs[&(pid, name.clone())];
                ctx.solver.adts[adt]
                    .impls
                    .insert(name, Field::new_constant(ty, ctx.location, dec));
            }
        }

        ctx.solver.ribs.pop();

        Ok(())
    }
}

impl Hoist for Pact {
    fn hoist(&self, ctx: HoistCtx) -> Result<()> {
        // push the empty pact and set it as `pact_self` before resolving signatures, so `Self`
        // and the pact's own name resolve to it within its own method types (see `resolve_name`)
        let pact_id = ctx.solver.pacts.push(shared::Pact {
            name: self.name.lexeme.clone(),
            functions: IndexMap::new(),
            constants: IndexMap::new(),
        });
        shared::name_pact(pact_id.index(), &self.name.lexeme);
        let prev_self = ctx.solver.pact_self.replace(pact_id);

        for item in self.items.iter() {
            match item {
                PactItem::Const { name, annotation } => {
                    let ty = Ty::from_annotation(annotation.clone(), ctx.solver)?;
                    ctx.solver.pacts[pact_id]
                        .constants
                        .insert(name.lexeme.clone(), ty);
                }
                PactItem::Fn {
                    name,
                    parameters,
                    return_type,
                    default,
                } => {
                    let is_method = parameters
                        .first()
                        .and_then(|b| b.left.as_ident())
                        .is_some_and(|i| i.lexeme == "self");

                    let default = default
                        .as_ref()
                        .map(|v| Ty::Vid(ctx.solver.node_vid(v.id())));

                    let header = hoist_fn_header(ctx.solver, parameters, return_type, is_method)?;
                    ctx.solver.pacts[pact_id]
                        .functions
                        .insert(name.lexeme.clone(), (header, default));
                }
            }
        }

        ctx.solver.pact_self = prev_self;

        // Mint a Dec for each defaulted method so the dispatch table can point at the shared
        // default when an impl omits it. Done at hoist so it exists before any body solves,
        // regardless of pact/impl source order.
        for item in self.items.iter() {
            if let PactItem::Fn {
                name,
                default: Some(body),
                ..
            } = item
            {
                let header = ctx.solver.pacts[pact_id].functions[&name.lexeme].0.clone();
                let dec = ctx.solver.dec_id(
                    name,
                    Ty::Fn(header),
                    DecKind::Item { defaults: vec![] },
                    ctx.vis,
                );
                ctx.solver
                    .pact_default_decs
                    .insert((pact_id, name.lexeme.clone()), dec);
                // def-site mapping: this is how compile finds the dec to lower the default
                // body into (emit.rs's Pact arm)
                ctx.solver.node_decs.insert(body.id(), dec);
            }
        }

        let dec = ctx.solver.dec_id(
            &self.name,
            Ty::pacts(vec![pact_id]),
            DecKind::Pact(pact_id),
            ctx.vis,
        );
        ctx.solver.ribs.module_mut().insert(self.name.clone(), dec);

        Ok(())
    }
}

impl Hoist for Function {
    fn hoist(&self, mut ctx: HoistCtx) -> Result<()> {
        let fn_header = hoist_fn_header(
            ctx.solver,
            &self.parameters,
            &self.return_type,
            self.is_method(),
        )?;

        let ty = Ty::Fn(fn_header);
        let node_id = ctx.node_id;
        ctx.write(&self.name, ty, false)?;

        // populate per-parameter defaults onto the just-created Item kind. defaults must be
        // const-reducible -- anything else is rejected here so the IR never has to deal with
        // a `None` literal where it expected a value. names live on the callee's `Ty::Fn`,
        // not here.
        if let Some(node_id) = node_id
            && let Some(&dec) = ctx.solver.node_decs.get(&node_id)
        {
            let defaults: Vec<Option<parse::Literal>> = self
                .parameters
                .iter()
                .map(|b| match b.right.as_ref() {
                    None => Ok(None),
                    Some(expr) => match ctx.solver.reduce_const_expr(expr)? {
                        Some(lit) => Ok(Some(lit)),
                        None => Err(crate::errors::NonConstDefault {
                            src: ctx.solver.src(expr.location()),
                            at: expr.location().into(),
                        })?,
                    },
                })
                .collect::<Result<_>>()?;
            if let DecKind::Item { defaults: slot } = &mut ctx.solver.decs[dec].kind {
                *slot = defaults;
            }
        }

        Ok(())
    }
}

impl Hoist for Const {
    fn hoist(&self, mut ctx: HoistCtx) -> Result<()> {
        let vid = ctx.solver.node_vid(self.right.id());
        ctx.write(&self.left, Ty::Vid(vid), true)
    }
}

fn hoist_fn_header(
    solver: &mut Solver,
    parameters: &[Binding],
    return_type: &Option<Annotation>,
    is_method: bool,
) -> Result<FnHeader> {
    let expected_ty = return_type
        .clone()
        .map_or(Ok(Ty::Unit), |a| Ty::from_annotation(a, solver))?;

    // Resolve each parameter's type into an `FnParam` for the `FnHeader`. `bind_parameter`
    // allocates one `Vid` per param and unifies it directly (no `Dec`, no rib insertion) --
    // the real per-body binding happens in `Function::solve`.
    let solved_parameters = parameters
        .iter()
        .map(|b| solver.bind_parameter(b))
        .collect::<Result<Vec<_>>>()?;

    // once a param has a default, every later one must too -- otherwise positional call sites
    // become ambiguous about where the default starts. `parameters` and `self.parameters`
    // are parallel, so an index serves both.
    if let Some(first) = solved_parameters.iter().position(|p| p.has_default) {
        for (i, param) in solved_parameters.iter().enumerate().skip(first + 1) {
            if !param.has_default {
                let loc = parameters[i].location();
                Err(crate::errors::RequiredAfterOptional {
                    src: solver.src(loc),
                    at: loc.into(),
                })?;
            }
        }
    }

    Ok(FnHeader::new(solved_parameters, expected_ty, is_method))
}
