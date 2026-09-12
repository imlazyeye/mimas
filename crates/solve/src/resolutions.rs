#![allow(unused)] // temp

use api::NativeId;
use indexmap::IndexMap;
use parse::{Literal, NodeId};
use shared::{IdVec, PactId};

use crate::{
    Solver,
    components::{Adt, AdtFlags, AdtId, DecId, DecKind, Ty, TyExt, Variant},
};

pub struct Resolutions {
    pub node_tys: IndexMap<NodeId, Ty>,
    pub node_decs: IndexMap<NodeId, DecId>,
    pub decs: IdVec<DecId, ResolvedDecl>,
    pub adts: IdVec<AdtId, ResolvedAdt>,
    pub closure_captures: IndexMap<NodeId, Vec<DecId>>,
}

#[derive(Debug, Clone)]
pub struct ResolvedDecl {
    pub name: String,
    pub kind: ResolvedDeclKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedDeclKind {
    Local,
    Item {
        defaults: Vec<Option<Literal>>,
        native: Option<NativeId>,
    },
    Constant(Literal),
    Variant {
        parent: AdtId,
        layout: AdtId,
    },
    Adt(AdtId),
    Pact(PactId),
}

pub struct ResolvedAdt {
    pub name: String,
    pub fields: Vec<String>,
    pub implements: Vec<PactId>,
    pub methods: IndexMap<String, DecId>,
    pub dispatch_ids: Vec<AdtId>,
}

impl ResolvedAdt {
    pub(crate) fn new(aid: AdtId, adt: &Adt, solver: &Solver) -> Self {
        let fields = match (adt.variants.len(), adt.variants.values().next()) {
            (1, Some(Variant::Struct(variant))) => variant.fields.keys().cloned().collect(),
            (1, Some(Variant::Tuple(variant))) => {
                (0..variant.members.len()).map(|i| i.to_string()).collect()
            }
            _ => Vec::new(),
        };

        let mut implements: Vec<PactId> = solver
            .pact_impls
            .iter()
            .filter_map(|(p, a)| (*a == aid).then_some(*p))
            .collect();
        implements.sort_by_key(|p| p.index());

        let methods = adt
            .impls
            .iter()
            .filter(|(_, field)| matches!(solver.decs[field.dec].kind, DecKind::Item { .. }))
            .map(|(name, field)| (name.clone(), field.dec))
            .collect();

        let dispatch_ids = if adt.flags.contains(AdtFlags::IS_ENUM) {
            adt.variants.values().filter_map(Variant::layout).collect()
        } else {
            vec![aid]
        };

        Self {
            name: adt.name.clone(),
            fields,
            implements,
            methods,
            dispatch_ids,
        }
    }
}

impl From<Solver> for Resolutions {
    fn from(solver: Solver) -> Self {
        let node_tys = solver
            .node_to_vid
            .iter()
            .map(|(node_id, vid)| (*node_id, Ty::Vid(*vid).normalized(&solver)))
            .collect();

        let mut resolved_adts = IdVec::new();
        for (aid, adt) in solver.adts.iter() {
            resolved_adts.push(ResolvedAdt::new(aid, adt, &solver));
        }

        let mut resolved_decs: IdVec<DecId, ResolvedDecl> = IdVec::new();
        for (id, dec) in solver.decs {
            let kind = match dec.kind {
                DecKind::Local | DecKind::LoopVar => ResolvedDeclKind::Local,
                DecKind::Item { defaults } => {
                    let native = solver.dec_to_native.get(&id).map(|b| b.id);
                    ResolvedDeclKind::Item { defaults, native }
                }
                DecKind::Constant(Some(lit)) => ResolvedDeclKind::Constant(lit),
                DecKind::Constant(None) => panic!(
                    "dec {:?} ({}) reached IR boundary as `DeclKind::Constant(None)`",
                    id, dec.name
                ),
                DecKind::Variant { parent, layout } => ResolvedDeclKind::Variant { parent, layout },
                DecKind::Adt(a) => ResolvedDeclKind::Adt(a),
                DecKind::Pact(p) => ResolvedDeclKind::Pact(p),
            };
            resolved_decs.push(ResolvedDecl {
                name: dec.name,
                kind,
            });
        }

        let closure_captures = solver
            .closure_captures
            .into_iter()
            .map(|(node, decs)| (node, decs.into_iter().collect()))
            .collect();

        Self {
            node_tys,
            node_decs: solver.node_decs,
            decs: resolved_decs,
            adts: resolved_adts,
            closure_captures,
        }
    }
}
