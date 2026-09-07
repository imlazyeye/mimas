use api::{Intrinsic, NativeId};
use itertools::Itertools;
use parse::NodeId;
use shared::{IdVec, Location, StrId, StrInterner};
use solve::{
    Resolutions,
    components::{DecId, Ty},
};
use std::collections::HashMap;

use crate::{
    IrDisplay,
    ir::{Block, BlockId, BlockWriter},
};

use super::{Body, BodyId, Inst, InstId, Local};

pub struct Ir {
    pub bodies: IdVec<BodyId, Body>,
    pub str_interner: StrInterner,
    pub item_bodies: HashMap<DecId, BodyId>,
    pub(crate) resolutions: Resolutions,
    pub(crate) intrinsics: HashMap<NativeId, Intrinsic>,

    pub(crate) current_body: BodyId,
    body_stack: Vec<BodyId>,
    pub(crate) current_loc: Location,
}

impl Ir {
    pub fn new(resolutions: Resolutions, intrinsics: HashMap<NativeId, Intrinsic>) -> Self {
        let root = Body::new();
        let mut bodies = IdVec::new();
        bodies.push(root);
        Self {
            bodies,
            resolutions,
            intrinsics,
            str_interner: StrInterner::new(),
            current_body: BodyId::ZERO,
            body_stack: Vec::new(),
            item_bodies: HashMap::new(),
            current_loc: Location::SYNTHETIC,
        }
    }

    pub(crate) fn with_loc<R>(&mut self, loc: Location, f: impl FnOnce(&mut Self) -> R) -> R {
        let prev = std::mem::replace(&mut self.current_loc, loc);
        let r = f(self);
        self.current_loc = prev;
        r
    }

    pub fn lower(&mut self, stmts: &[parse::Stmt]) {
        for stmt in stmts {
            self.stmt(stmt);
        }
        // the entry body implicitly returns null; emit it explicitly so its final block ends in a
        // terminator like every other body -- codegen no longer synthesizes one.
        if !self.is_terminated() {
            let null = self.current().constant(crate::Constant::Null);
            self.current().ret(null);
        }
    }

    // pub fn ir_display(&self) -> String {
    //     self.functions
    //         .iter()
    //         .map(|f| f.ir_display(self))
    //         .collect::<Vec<_>>()
    //         .join("\n")
    // }

    pub(crate) fn is_terminated(&self) -> bool {
        let inst = self
            .current_body()
            .current_stream()
            .last()
            .map(|v| &self.current_body().instructions[v]);

        matches!(
            inst,
            Some(Inst::Jump { .. })
                | Some(Inst::JumpIfFalse { .. })
                | Some(Inst::Return { .. })
                | Some(Inst::Switch { .. })
        )
    }

    pub(crate) fn in_body<T>(&mut self, body: BodyId, emit: impl FnOnce(&mut Ir) -> T) -> T {
        self.body_stack.push(self.current_body);
        self.current_body = body;
        let out = emit(self);
        self.current_body = self.body_stack.pop().unwrap();
        out
    }

    pub(crate) fn push_block(&mut self, name: &str) -> BlockId {
        let block = Block::new();
        let id = self.current_body_mut().blocks.push(block);

        cfg_select! {
            feature = "logging" => {
                self.current_body_mut().block_names.insert(id, name.into());
            }

            _ => {
                let _name = name;
            }
        }

        id
    }

    pub(crate) fn target(&mut self, id: BlockId) {
        self.current_body_mut().current_block = id;
    }

    pub(crate) fn current_block_id(&self) -> BlockId {
        self.current_body().current_block
    }

    pub(crate) fn in_block<T>(
        &mut self,
        block: BlockId,
        emit: impl FnOnce(&mut BlockWriter<'_>) -> T,
    ) -> T {
        let old = self.current_block_id();
        self.target(block);
        let out = self.in_current(emit);
        self.target(old);
        out
    }

    pub(crate) fn in_current<T>(&mut self, emit: impl FnOnce(&mut BlockWriter<'_>) -> T) -> T {
        emit(&mut BlockWriter { ir: self })
    }

    pub(crate) fn current(&mut self) -> BlockWriter<'_> {
        BlockWriter { ir: self }
    }

    pub(super) fn loop_stack_mut(&mut self) -> &mut Vec<LoopCtx> {
        &mut self.current_body_mut().loop_stack
    }

    pub(super) fn push_inst(&mut self, inst: impl Into<Inst>) -> InstId {
        let loc = self.current_loc;
        let body = self.current_body_mut();
        let iid = body.instructions.push(inst.into());
        body.locs.push(loc);
        body.current_stream_mut().push(iid);
        iid
    }

    pub(crate) fn str(&self, id: StrId) -> &str {
        self.str_interner.get(id)
    }

    pub(super) fn intern_str(&mut self, s: &str) -> StrId {
        self.str_interner.intern(s)
    }

    pub(crate) fn try_node_dec(&self, node: NodeId) -> Option<DecId> {
        self.resolutions.node_decs.get(&node).copied()
    }

    pub(crate) fn node_dec(&self, node: NodeId) -> DecId {
        self.try_node_dec(node)
            .unwrap_or_else(|| panic!("no resolved DecId for node {node:?}"))
    }

    // struct fields are positional (declaration order, matching how instances are built);
    // resolve a named field on `receiver`'s type to its slot. unwraps an option layer if the
    // access was an optional `?.` -- the solver typed it that way but the slot lookup needs the
    // inner adt.
    pub(crate) fn field_index(&self, receiver: NodeId, field: &str) -> usize {
        let adt = match self.resolutions.node_tys.get(&receiver) {
            Some(Ty::Adt(adt)) | Some(Ty::Identity(adt)) => *adt,
            Some(Ty::Option(inner)) => match inner.as_ref() {
                Ty::Adt(adt) | Ty::Identity(adt) => *adt,
                other => panic!("field access on option of non-adt type: {other:?}"),
            },
            other => panic!("field access on non-adt type: {other:?}"),
        };
        self.resolutions.adts[adt]
            .fields
            .iter()
            .position(|n| n == field)
            .unwrap_or_else(|| panic!("no field `{field}` on adt"))
    }

    // allocate-or-fetch this binding's slot within the current body
    pub(crate) fn local_for(&mut self, dec: DecId) -> Local {
        if let Some(&local) = self.bodies[self.current_body].dec_to_local.get(&dec) {
            return local;
        }
        let name = self.resolutions.decs[dec].name.to_string();
        let body = &mut self.bodies[self.current_body];
        let local = body.locals.push(dec);
        body.dec_to_local.insert(dec, local);
        body.artifacts.insert(local, name);
        local
    }

    pub(crate) fn synthetic_local(&mut self, name: &str) -> Local {
        let body = &mut self.bodies[self.current_body];
        let local = body.locals.push(DecId::DANGLING);
        body.artifacts.insert(local, name.to_string());
        local
    }

    // allocate-or-fetch the body associated with this dec
    pub(crate) fn item_body_for(&mut self, dec: DecId) -> BodyId {
        if let Some(&bid) = self.item_bodies.get(&dec) {
            return bid;
        }
        let bid = self.bodies.push(Body::new());
        self.item_bodies.insert(dec, bid);
        bid
    }

    pub(super) fn current_body(&self) -> &Body {
        &self.bodies[self.current_body]
    }

    pub(super) fn current_body_mut(&mut self) -> &mut Body {
        &mut self.bodies[self.current_body]
    }
}

#[derive(Debug)]
pub struct LoopCtx {
    pub(crate) continuation: BlockId,
    pub(crate) exit: BlockId,
    pub(crate) breaks: Vec<(BlockId, InstId)>,
    pub(crate) continues: Vec<BlockId>,
    pub(crate) collection: Option<InstId>,
    pub(crate) collects: Vec<BlockId>,
}

impl LoopCtx {
    pub(super) fn new(continuation: BlockId, exit: BlockId, collection: Option<InstId>) -> Self {
        Self {
            continuation,
            exit,
            breaks: Vec::new(),
            continues: Vec::new(),
            collection,
            collects: Vec::new(),
        }
    }
}

#[mutants::skip]
impl std::fmt::Display for Ir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // look upon my power
        write!(
            f,
            "{}",
            self.bodies
                .iter()
                .map(|(body_id, body)| {
                    std::iter::once(
                        (body_id != BodyId::ZERO).then(|| format!("body @{}", body_id.index())),
                    )
                    .flatten()
                    .chain(body.blocks.iter().map(|(block_id, block)| {
                        let instructions = block
                            .stream
                            .iter()
                            .map(|iid| {
                                format!(
                                    "    [ i{} ] {}",
                                    iid.index(),
                                    body.instructions[iid].ir_display(self),
                                )
                            })
                            .join("\n");

                        cfg_select! {
                            feature = "logging" => {
                                let name = body.block_names.get(&block_id).unwrap();
                            }
                            _ => {
                                let name = format!("b{}", block_id.index());
                            }
                        };

                        if instructions.is_empty() {
                            format!("{name}:")
                        } else {
                            format!("{name}:\n{instructions}")
                        }
                    }))
                    .join("\n")
                })
                .join("\n")
        )
    }
}

// impl IrDisplay for Body {
//     fn ir_display(&self, ir: &Ir) -> String {
//         let header = match self.name {
//             Some(n) => format!("fn @{} {}:", self.id.index(), ir.str(n)),
//             None if self.id.index() == 0 => String::new(), // for the root
//             None => format!("fn @{}:", self.id.index()),
//         };
//         let blocks = self
//             .blocks
//             .iter()
//             .map(|block| block.ir_display(ir))
//             .collect::<Vec<_>>()
//             .join("\n");
//         if header.is_empty() {
//             blocks
//         } else {
//             format!("{header}\n{blocks}")
//         }
//     }
// }
