#![allow(unused)] // temp

use std::collections::HashMap;

use parse::{Expr, ExprKind, Ident, NodeId};
use shared::{AdtId, Location, PactId};

use crate::components::{Ty, Vid};

#[derive(Debug, Clone)]
pub(crate) struct Ribs {
    inner: Vec<Rib>,
}

impl Default for Ribs {
    fn default() -> Self {
        Self {
            inner: vec![Rib::new(RibKind::Module(AdtId::DANGLING))],
        }
    }
}

impl Ribs {
    pub(crate) fn push_module(&mut self, adt: AdtId) {
        self.inner.push(Rib::new(RibKind::Module(adt)));
    }

    /// AdtId of the innermost module rib. `AdtId::DANGLING` for the implicit root rib
    /// (no `module` decl) -- treat that as "no module" for visibility purposes.
    pub(crate) fn current_module(&self) -> AdtId {
        self.inner
            .iter()
            .rev()
            .find_map(|r| match r.kind {
                RibKind::Module(a) => Some(a),
                _ => None,
            })
            .unwrap_or(AdtId::DANGLING)
    }

    pub(crate) fn push_rib(&mut self, rib: Rib) {
        self.inner.push(rib);
    }

    pub(crate) fn push_import(&mut self) {
        self.inner.push(Rib::new(RibKind::Import));
    }

    pub(crate) fn push_block(&mut self) {
        self.inner.push(Rib::new(RibKind::Block));
    }

    pub(crate) fn push_function(&mut self) {
        self.inner.push(Rib::new(RibKind::Function));
    }

    pub(crate) fn push_closure(&mut self, node_id: NodeId) {
        self.inner.push(Rib::new(RibKind::Closure(node_id)));
    }

    pub(crate) fn push_impl(&mut self, adt_id: AdtId) {
        self.inner.push(Rib::new(RibKind::Impl(adt_id)));
    }

    pub(crate) fn pop(&mut self) -> Rib {
        self.inner.pop().expect("tried to pop the root module rib")
    }

    pub(crate) fn current_mut(&mut self) -> &mut Rib {
        self.inner.last_mut().unwrap()
    }

    pub(crate) fn import_mut(&mut self) -> &mut Rib {
        self.inner
            .iter_mut()
            .rev()
            .find(|rib| rib.kind == RibKind::Import)
            .expect("no import rib in scope")
    }

    /// The active module scope that holds hoisted items.
    pub(crate) fn module_mut(&mut self) -> &mut Rib {
        self.inner
            .iter_mut()
            .rev()
            .find(|rib| matches!(rib.kind, RibKind::Module(_)))
            .expect("no module rib in scope")
    }

    /// Looks a name up in the active module scope only.
    pub(crate) fn module_lookup(&self, ident: &Ident) -> Option<DecId> {
        self.inner
            .iter()
            .rev()
            .find(|rib| matches!(rib.kind, RibKind::Module(_)))
            .and_then(|rib| rib.get(ident))
    }

    /// Resolves a name from the innermost scope outward. Locals stop being visible past the first
    /// function boundary, module items and imports never do.
    pub(crate) fn resolve(&self, ident: &Ident) -> Option<DecId> {
        let mut crossed_fn = false;
        for rib in self.inner.iter().rev() {
            if let Some(binding) = rib.get(ident)
                && (rib.kind.visible_across_functions() || !crossed_fn)
            {
                return Some(binding);
            }
            if matches!(rib.kind, RibKind::Function) {
                crossed_fn = true;
            }
        }
        None
    }

    /// Like [`resolve`], but also reports the closure boundaries crossed on the way out -- used by
    /// `Ident::solve` to record captures.
    pub(crate) fn resolve_with_closures(&self, ident: &Ident) -> Option<(DecId, Vec<NodeId>)> {
        let mut crossed_fn = false;
        let mut crossed_closures = Vec::new();
        for rib in self.inner.iter().rev() {
            if let Some(binding) = rib.get(ident)
                && (rib.kind.visible_across_functions() || !crossed_fn)
            {
                return Some((binding, crossed_closures));
            }
            match rib.kind {
                RibKind::Function => crossed_fn = true,
                RibKind::Closure(id) => crossed_closures.push(id),
                _ => {}
            }
        }
        None
    }

    /// Looks a name up in the innermost rib only.
    pub(crate) fn resolve_current_only(&self, ident: &Ident) -> Option<DecId> {
        self.inner.last().and_then(|rib| rib.get(ident))
    }

    pub(crate) fn impl_target(&self) -> Option<AdtId> {
        self.inner.iter().rev().find_map(|v| {
            if let RibKind::Impl(adt) = v.kind {
                Some(adt)
            } else {
                None
            }
        })
    }

    pub(crate) fn pop_module(&mut self) -> Rib {
        let module = self.pop();
        assert!(matches!(module.kind, RibKind::Module(_)));
        module
    }

    pub(crate) fn pop_import(&mut self) -> Rib {
        let imports = self.pop();
        assert_eq!(imports.kind, RibKind::Import);
        imports
    }

    pub(crate) fn pop_block(&mut self) -> Rib {
        let block = self.pop();
        assert_eq!(block.kind, RibKind::Block);
        block
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Rib {
    pub(crate) table: HashMap<String, DecId>,
    pub(crate) kind: RibKind,
}

impl Rib {
    pub(crate) fn new(kind: RibKind) -> Self {
        Self {
            table: HashMap::new(),
            kind,
        }
    }

    pub(crate) fn insert(&mut self, ident: Ident, dec_id: DecId) {
        self.table.insert(ident.lexeme, dec_id);
    }

    fn get(&self, ident: &Ident) -> Option<DecId> {
        self.table.get(&ident.lexeme).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum RibKind {
    /// The root scope. Holds hoisted items; stays visible across function boundaries.
    /// Carries the file's module `AdtId`, or `AdtId::DANGLING` for the implicit/no-file root
    /// rib -- used to attribute decs to their owning module for visibility checks.
    Module(AdtId),
    /// A file/module-level import scope. Imports are visible across function boundaries, but are
    /// not exported when a module rib is closed into an ADT.
    Import,
    /// An ordinary lexical scope: a block, loop, match arm, or closure body.
    Block,
    /// A function body. Names bound outside it cannot be captured from within.
    Function,
    /// A closure body. Same visibility as a block, but identified so resolutions that cross it
    /// can be recorded as captures of the closure expression with this NodeId.
    Closure(NodeId),
    /// An impl body. Defines the current Adt that Self refers to
    Impl(AdtId),
}

impl RibKind {
    fn visible_across_functions(self) -> bool {
        matches!(self, Self::Module(_) | Self::Import)
    }
}

shared::id!(pub DecId);

#[derive(Debug, Clone)]
pub struct Dec {
    pub name: String,
    pub location: Location,
    pub kind: DecKind,
    pub vid: Vid,
    pub vis: Vis,
    pub module: AdtId,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecKind {
    Local,
    Item {
        defaults: Vec<Option<parse::Literal>>,
    },
    Constant(Option<parse::Literal>),
    Variant {
        parent: AdtId,
        layout: AdtId,
    },
    Adt(AdtId),
    Pact(PactId),
    LoopVar,
}

impl DecKind {
    pub fn is_constant(&self) -> bool {
        matches!(self, DecKind::Constant(_))
    }

    pub fn const_value(&self) -> Option<&parse::Literal> {
        match self {
            DecKind::Constant(Some(lit)) => Some(lit),
            _ => None,
        }
    }

    pub fn as_adt(&self) -> Option<AdtId> {
        match self {
            DecKind::Adt(a) => Some(*a),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Vis {
    #[default]
    Private,
    Public,
}
