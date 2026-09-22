#![allow(unused)] // temp

use std::{collections::HashMap, sync::Arc};

use api::NativeId;
use indexmap::IndexMap;
use miette::NamedSource;
use parse::{Literal, NodeId};
use shared::{FileId, IdVec, Location, PactId, TyNames};

use crate::{
    Solver,
    components::{Adt, AdtFlags, AdtId, DecId, DecKind, Ty, TyExt, Variant, Vis},
};

pub struct Resolutions {
    pub node_tys: IndexMap<NodeId, Ty>,
    pub node_decs: IndexMap<NodeId, DecId>,
    pub decs: IdVec<DecId, ResolvedDecl>,
    pub adts: IdVec<AdtId, ResolvedAdt>,
    pub pact_names: IdVec<PactId, String>,
    pub module_paths: HashMap<AdtId, Vec<String>>,
    pub closure_captures: IndexMap<NodeId, Vec<DecId>>,
    pub root: ResolvedModule,
    pub sources: HashMap<FileId, NamedSource<Arc<str>>>,
}

impl Resolutions {
    /// An adt's name qualified by the module it's declared in, e.g. `one::two::Foo`.
    pub fn adt_path(&self, aid: AdtId) -> String {
        let adt = &self.adts[aid];
        let mut path = self.module_path(adt.module).to_vec();
        path.push(Ty::Adt(aid).display(self));
        path.join("::")
    }

    /// What a dec sits in, as written before its name: the adt for a method or field, the
    /// module path for anything else (empty at the root).
    pub fn owner_path(&self, dec: DecId) -> String {
        let dec = &self.decs[dec];
        match dec.owner {
            Some(owner) => self.adt_path(owner),
            None => self.module_path(dec.module).join("::"),
        }
    }

    /// The segments leading to `module`. Empty for the root (or anything that isn't a module).
    pub fn module_path(&self, module: AdtId) -> &[String] {
        self.module_paths.get(&module).map_or(&[], Vec::as_slice)
    }
}

impl TyNames for Resolutions {
    fn adt(&self, id: AdtId) -> Option<String> {
        self.adts.get(id).map(|adt| adt.name.clone())
    }

    fn pact(&self, id: PactId) -> Option<String> {
        self.pact_names.get(id).cloned()
    }
}

/// Everything a file or module declares, in declaration order.
#[derive(Debug, Clone, Default)]
pub struct ResolvedModule {
    pub items: IndexMap<String, DecId>,
    pub modules: IndexMap<String, ResolvedModule>,
}

impl ResolvedModule {
    fn new(solver: &Solver, adt: AdtId) -> Self {
        let root = adt == AdtId::DANGLING;
        let items = solver
            .module_items
            .get(&adt)
            .into_iter()
            .flatten()
            .map(|(name, &dec)| (name.clone(), dec))
            .collect();
        let children: Vec<(String, AdtId)> = if root {
            solver
                .root_modules
                .iter()
                .map(|(name, &adt)| (name.clone(), adt))
                .collect()
        } else {
            let fields = solver.adts[adt].as_struct().fields.iter();
            fields
                .filter_map(|(name, field)| {
                    let child = *field.ty.as_adt()?;
                    let is_module = solver.adts[child].flags.contains(AdtFlags::IS_MODULE);
                    is_module.then(|| (name.clone(), child))
                })
                .collect()
        };
        Self {
            items,
            modules: children
                .into_iter()
                .map(|(name, child)| (name, Self::new(solver, child)))
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedDecl {
    pub name: String,
    pub ty: Ty,
    pub kind: ResolvedDeclKind,
    pub vis: Vis,
    pub location: Location,
    pub module: AdtId,
    pub owner: Option<AdtId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedDeclKind {
    Local,
    Item {
        defaults: Vec<Option<Literal>>,
        native: Option<NativeId>,
        takes_self: bool,
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
    pub module: AdtId,
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

        let module = solver
            .decs
            .iter()
            .find(|(_, dec)| dec.kind == DecKind::Adt(aid))
            .map_or(AdtId::DANGLING, |(_, dec)| dec.module);

        Self {
            name: adt.name.clone(),
            module,
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

        let root = ResolvedModule::new(&solver, AdtId::DANGLING);

        fn module_paths(
            solver: &Solver,
            adt: AdtId,
            prefix: &[String],
            out: &mut HashMap<AdtId, Vec<String>>,
        ) {
            for (name, field) in solver.adts[adt].as_struct().fields.iter() {
                let Some(child) = field.ty.as_adt().copied() else {
                    continue;
                };
                if !solver.adts[child].flags.contains(AdtFlags::IS_MODULE) {
                    continue;
                }
                let mut path = prefix.to_vec();
                path.push(name.clone());
                out.insert(child, path.clone());
                module_paths(solver, child, &path, out);
            }
        }
        let mut paths = HashMap::new();
        for (name, &adt) in &solver.root_modules {
            paths.insert(adt, vec![name.clone()]);
            module_paths(&solver, adt, std::slice::from_ref(name), &mut paths);
        }

        let mut owners: HashMap<DecId, AdtId> = HashMap::new();
        for (aid, adt) in solver.adts.iter() {
            for field in adt.impls.values() {
                owners.insert(field.dec, aid);
            }
            if let Some(Variant::Struct(variant)) = adt.variants.values().next() {
                for field in variant.fields.values() {
                    owners.insert(field.dec, aid);
                }
            }
        }

        let tys: Vec<Ty> = solver
            .decs
            .iter()
            .map(|(_, dec)| Ty::Vid(dec.vid).normalized(&solver))
            .collect();

        let mut resolved_decs: IdVec<DecId, ResolvedDecl> = IdVec::new();
        for ((id, dec), ty) in solver.decs.into_iter().zip(tys) {
            let vis = dec.vis;
            let kind = match dec.kind {
                DecKind::Local | DecKind::LoopVar => ResolvedDeclKind::Local,
                DecKind::Item { defaults } => ResolvedDeclKind::Item {
                    defaults,
                    native: solver.dec_to_native.get(&id).map(|b| b.id),
                    takes_self: match solver.dec_to_native.get(&id) {
                        Some(binding) => binding.takes_self,
                        None => matches!(&ty, Ty::Fn(header) if header.is_method),
                    },
                },
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
                ty,
                kind,
                vis,
                location: dec.location,
                module: dec.module,
                owner: owners.get(&id).copied(),
            });
        }

        let mut pact_names = IdVec::new();
        for (_, pact) in solver.pacts.iter() {
            pact_names.push(pact.name.clone());
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
            pact_names,
            module_paths: paths,
            closure_captures,
            root,
            sources: solver.sources,
        }
    }
}
