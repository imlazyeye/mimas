use shared::IdVec;

use colored::Colorize;
use parse::Literal;
use shared::{FileId, Location, Ty};
use solve::{
    ResolvedDeclKind, ResolvedModule,
    components::{DecId, Vis},
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    BinOp, BlockId, BlockTarget, BodyId, Constant, Function, Inst, InstId, Ir, Local, Module, Op,
    OperandKind,
    codegen::{
        clean::{self, uses},
        program::{Chunk, Encoder, Program},
    },
    ir::Body,
};

shared::id!(pub Reg);

#[derive(Debug)]
pub struct Compiler {
    pub(crate) chunks: IdVec<BodyId, Chunk>,
    bytes: Encoder,
    signatures: IdVec<BodyId, Option<Function>>,
    disasm: bool,
    srcs: HashMap<FileId, Arc<str>>,
}

// fuse a comparison op + JumpIfFalse into its `B`-prefixed branch form. reg-reg comparisons carry
// `left, right`; immediate comparisons carry `left, val` -- list them in the two groups.
macro_rules! fuse_branch {
    ($cmp:expr, $target:expr;
     $($rfrom:ident => $rto:ident),* ;
     $($ifrom:ident => $ito:ident),* $(,)?) => {
        match $cmp {
            $(Op::$rfrom { left, right, .. } => Op::$rto { target: $target, left, right, is_true: false },)*
            $(Op::$ifrom { left, val, .. } => Op::$ito { target: $target, left, val, is_true: false },)*
            _ => unreachable!("incorrectly marked an op as a branchable comp"),
        }
    };
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunks: IdVec::new(),
            bytes: Encoder::new(),
            signatures: IdVec::new(),
            disasm: false,
            srcs: HashMap::new(),
        }
    }

    pub fn with_disasm(mut self, on: bool) -> Self {
        self.disasm = on;
        self
    }

    /// Source text per file, used only to interleave source lines into the `--dump-bytes` disasm.
    pub fn with_sources(mut self, srcs: HashMap<FileId, Arc<str>>) -> Self {
        self.srcs = srcs;
        self
    }

    pub fn compile(&mut self, ir: &mut Ir) -> Program {
        let first_new = self.chunks.len();

        // body -> source name, for the --disasm dump only
        let names: HashMap<BodyId, String> = if self.disasm {
            ir.item_bodies
                .iter()
                .map(|(&dec, &body)| (body, ir.resolutions.decs[dec].name.clone()))
                .collect()
        } else {
            HashMap::new()
        };

        /// A dec the host can call: everything but a native, which has no body of its own. Hands
        /// back the `Function` already built for `signatures` instead of building a second one, so
        /// the two views of the same dec can't drift apart.
        fn exported_fn(
            ir: &Ir,
            signatures: &IdVec<BodyId, Option<Function>>,
            dec: DecId,
        ) -> Option<Function> {
            let body = *ir.item_bodies.get(&dec)?;
            signatures.get(body)?.clone()
        }

        fn export(
            ir: &mut Ir,
            signatures: &IdVec<BodyId, Option<Function>>,
            module: ResolvedModule,
        ) -> Module {
            let mut out = Module::default();
            for (name, dec) in module.items {
                match ir.resolutions.decs[dec].kind.clone() {
                    ResolvedDeclKind::Item { .. } => {
                        let f = exported_fn(ir, signatures, dec)
                            .unwrap_or_else(|| unreachable!("item `{name}` is not a callable fn"));
                        out.functions.insert(name, f);
                    }
                    // dicts and structs have no constant form, see `Constant::emit_const_dec`
                    ResolvedDeclKind::Constant(lit)
                        if !matches!(lit, Literal::Dictionary(_) | Literal::Struct(_)) =>
                    {
                        out.constants.insert(name, Constant::from_literal(ir, lit));
                    }
                    ResolvedDeclKind::Adt(adt) => {
                        let resolved = &ir.resolutions.adts[adt];
                        let fields = resolved.fields.clone();
                        // the solver keeps impls in a `HashMap`, whose order is randomized per
                        // process -- sort so the export reads the same on every run
                        let mut methods: Vec<(String, DecId)> = resolved
                            .methods
                            .iter()
                            .map(|(name, &dec)| (name.clone(), dec))
                            .collect();
                        methods.sort_by(|(a, _), (b, _)| a.cmp(b));
                        let methods = methods
                            .into_iter()
                            .filter_map(|(name, dec)| {
                                Some((name, exported_fn(ir, signatures, dec)?))
                            })
                            .collect();
                        out.types.insert(
                            name,
                            crate::Type {
                                vis: ir.resolutions.decs[dec].vis,
                                fields,
                                methods,
                            },
                        );
                    }
                    _ => {}
                }
            }
            for (name, module) in module.modules {
                out.modules.insert(name, export(ir, signatures, module));
            }
            out
        }
        // every body's own signature keyed by body rather than by dec so a function value can be
        // called from a `BodyId` alone. a closure's defaults are never reachable from a call site
        // that only has the value, so it reports none -- same as what a dynamic call lowers to.
        while self.signatures.len() < ir.bodies.len() {
            self.signatures.push(None);
        }
        // read rather than drained -- `export` still needs `item_bodies` to find each dec's body
        let item_bodies: Vec<(DecId, BodyId)> = ir
            .item_bodies
            .iter()
            .map(|(&dec, &body)| (dec, body))
            .collect();
        for (dec, body) in item_bodies {
            let ResolvedDeclKind::Item { defaults, .. } = ir.resolutions.decs[dec].kind.clone()
            else {
                continue;
            };
            let Ty::Fn(header) = ir.resolutions.decs[dec].ty.clone() else {
                continue;
            };
            let vis = ir.resolutions.decs[dec].vis;
            let defaults = defaults
                .into_iter()
                .map(|d| d.map(|lit| Constant::from_literal(ir, lit)))
                .collect();
            self.signatures[body] = Some(Function {
                body,
                vis,
                header,
                defaults,
            });
        }
        for (body, node) in std::mem::take(&mut ir.closure_bodies) {
            let Some(Ty::Fn(header)) = ir.resolutions.node_tys.get(&node).cloned() else {
                continue;
            };
            let defaults = vec![None; header.parameters.len()];
            self.signatures[body] = Some(Function {
                body,
                vis: Vis::Private,
                header,
                defaults,
            });
        }

        let root = std::mem::take(&mut ir.resolutions.root);
        let root = export(ir, &self.signatures, root);

        for (body_id, body) in ir.bodies.iter_mut().skip(first_new) {
            // body compilation happens in 9 stages
            //
            // 1. clean      -- dce, phi threading, etc
            // 2. local regs -- assign every local a register
            // 3. phi regs   -- assign every phi a reg now that locals are known
            // 4. constants  -- decide which constants to inline or pin
            // 5. order      -- determine the order blocks are written in
            // 6. liveness   -- compute when every value is needed
            // 7. positions  -- use stage 6 to compute where every value dies
            // 8. copies     -- detach reads that cannot safely share the local's reg
            // 9. emit       -- walk over it all and write the ops
            //
            // stage 1: clean
            clean::clean(body);

            // stage 2: allocate a register for every single local. these go first so that the
            // register of a local is always aligned with 0..bodies.locals.len(). we use this at the
            // emit stage to ask if a given register is a local
            let mut regs: IdVec<Reg, ()> = IdVec::new();
            let mut local_to_reg: IdVec<Local, Reg> = IdVec::new();
            body.locals.iter().for_each(|_| {
                local_to_reg.push(regs.push(()));
            });

            let mut inst_to_reg: IdVec<InstId, Option<Reg>> =
                vec![None; body.instructions.len()].into();

            // stage 3: phi registers. phis are ephemeral, representing the value from whatever
            // block we just came from. note that a phi's operand is read at the _end of its
            // predecessor_, not within the block the phi sits in. that matters for liveness!
            let mut phi_copies: HashMap<BlockId, Vec<(Reg, InstId)>> = HashMap::new();

            for (_, block) in body.blocks.iter() {
                for &iid in &block.stream {
                    if let Inst::Phi(branches) = &body.instructions[iid] {
                        let dst = regs.push(());
                        inst_to_reg[iid] = Some(dst);
                        for &(pred, value) in branches {
                            phi_copies.entry(pred).or_default().push((dst, value));
                        }
                    }
                }
            }

            let chunk_offset = self.bytes.len();
            let mut byte_offset = chunk_offset;
            let mut body_ops = Vec::new();
            // used only for --dump-bytes, screw your compile times
            let mut freed_after: HashMap<usize, Vec<Reg>> = HashMap::new();
            let mut hidden: HashSet<InstId> = HashSet::new();
            let mut block_offset: IdVec<BlockId, usize> = IdVec::from(vec![0; body.blocks.len()]);
            let mut locs: Vec<(u32, Location)> = Vec::new();
            let push_loc = |off: usize, loc: Location, locs: &mut Vec<(u32, Location)>| {
                let rel = u32::try_from(off - chunk_offset).unwrap();
                if locs.last().is_none_or(|(_, prev)| *prev != loc) {
                    locs.push((rel, loc));
                }
            };

            // a constant whose every use is an arithmetic immediate is never materialized -- the
            // value is inlined into each AddIntImm/etc. rather than loaded into a register.
            let absorbed: HashSet<InstId> = {
                let mut total: HashMap<InstId, u32> = HashMap::new();
                let mut as_imm: HashMap<InstId, u32> = HashMap::new();
                for (_, block) in body.blocks.iter() {
                    for &iid in &block.stream {
                        let inst = &body.instructions[iid];
                        let absorbs = imm_binop(body, inst).map(|(_, con, _, _, _)| con);
                        for u in uses(inst) {
                            if matches!(
                                body.instructions[u],
                                Inst::Constant(Constant::Int(_) | Constant::Float(_))
                            ) {
                                *total.entry(u).or_default() += 1;
                                if absorbs == Some(u) {
                                    *as_imm.entry(u).or_default() += 1;
                                }
                            }
                        }
                    }
                }
                total
                    .iter()
                    .filter(|(c, n)| as_imm.get(c) == Some(n))
                    .map(|(&c, _)| c)
                    .collect()
            };

            // stage 4: give each DISTINCT cache-friendly constant a pinned register, so
            // loop-resident loads can be hoisted to the prologue and every use just aliases the
            // reg.
            //
            // this is currently limited to 12, as in theory, one could have a hot function with
            // a body like this...
            // ```
            // if foo {
            //     // many many constants
            // } else {
            //     // many many OTHER constants
            // }
            // ```
            //
            // ... where lifting the wrong branches constants would be a wasteful expense. as time
            // goes on though, this continues to not come up whereas a reason to lift the cap does.
            // so, this might raise in the future, perhaps to a much higher number.
            let mut constants = HashMap::new();
            'outer: for (_, block) in &body.blocks {
                for iid in block.stream.iter().copied() {
                    if let Inst::Constant(con) = &body.instructions[iid]
                        && let Some(con) = con.cached()
                        && !absorbed.contains(&iid)
                    {
                        constants.entry(con).or_insert_with(|| regs.push(()));
                        if constants.len() >= 12 {
                            break 'outer;
                        }
                    }
                }
            }

            // get the emit order of each block so that we can identify which jumps are
            // fallthroughs. we can't just look at block index + 1 because dce could have made gaps.
            let successors = |bid: BlockId| {
                let mut fallthrough = None;
                let mut branches = Vec::new();
                for &iid in &body.blocks[bid].stream {
                    match &body.instructions[iid] {
                        Inst::Jump { target } => fallthrough = Some(*target),
                        Inst::JumpIfFalse { target, .. } | Inst::ForNext { target, .. } => {
                            branches.push(*target)
                        }
                        Inst::Switch { table, default, .. } => {
                            branches.extend(table.iter().copied());
                            branches.push(*default);
                        }
                        _ => {}
                    }
                }
                (fallthrough, branches)
            };

            // stage 5: get the order we'll write the blocks in. depth-first walk from the entry so
            // that unreachable blocks are never visited. we guarantee that the _makes_ of values
            // always occur before they are ever _read_, so that running in the opposite direction
            // can support the liveness stage later.
            let mut order = Vec::new();
            let mut placed = vec![false; body.blocks.len()];
            let mut stack = vec![BlockId::ZERO];
            while let Some(b) = stack.pop() {
                // give me take!!!! why did they reject the rfc!!!
                if std::mem::replace(&mut placed[b.index()], true) {
                    continue;
                }

                order.push(b);
                let (fallthrough, branches) = successors(b);
                for b in branches {
                    if !placed[b.index()] {
                        stack.push(b);
                    }
                }

                if let Some(f) = fallthrough
                    && !placed[f.index()]
                {
                    stack.push(f);
                }
            }

            // stage 6: compute the liveness of values. a value is "live" at a given point in the
            // code if something later will still read it. at this stage we only notate the values
            // that are alive in a given block. later, we'll boil that down to an actual position.
            //
            // to compute this, we walk a block from its last instruction to its first, carrying a
            // set. since we're going backwards, this means we can add something to our set whenever
            // it is read, and remove it whenever it is made. when reversed again, the accurate
            // lifespan of a value is left.
            //
            // note that this requires the while loop because the final block in a loop (B) jumps
            // back to the first (A), which means that A depends on B, and B depends on A. no
            // visiting order can get that in one pass, so we repeat until no more diffs are found.
            let mut live_in: HashMap<BlockId, HashSet<InstId>> = HashMap::new();

            // #58: a GetLocal emits nothing. its value just "lives" in the local's register. that
            // is only true as long as nobody _sets_ that local. if a SetLocal to `x` runs while
            // an earlier read of `x` is still live, whoever uses that read gets the new value.
            // for example, with `x + { x = 5; 1 }`, the read of `x` is live from before the block
            // until the end of the add, causing the add to use `5` instead of whatever x's previous
            // value was. to avoid this, we use CopyLocal to create a safe barrier when the pattern
            // is encountered.
            let mut needs_copy: HashSet<InstId> = HashSet::new();

            let mut changed = true;
            while changed {
                changed = false;
                for &bid in order.iter().rev() {
                    let (fallthrough, branches) = successors(bid);
                    let mut live = HashSet::new();
                    for succ in fallthrough.into_iter().chain(branches) {
                        live.extend(live_in.get(&succ).into_iter().flatten().copied());
                    }
                    // a phi reads its operand at the end of the predecessor
                    live.extend(phi_copies.get(&bid).into_iter().flatten().map(|(_, v)| *v));
                    for &iid in body.blocks[bid].stream.iter().rev() {
                        // this instruction makes `iid`, so nothing above this line can need it
                        live.remove(&iid);
                        match &body.instructions[iid] {
                            // phis are counted at their corresponding tail -- we avoid counting
                            // them twice here
                            Inst::Phi(_) => continue,
                            // see above note about #58/copies
                            Inst::SetLocal(l, _) => needs_copy.extend(live.iter().filter(
                                |v| matches!(body.instructions[**v], Inst::GetLocal(r) if r == *l),
                            )),
                            _ => {}
                        }
                        live.extend(uses(&body.instructions[iid]));
                    }
                    // only a change at the top of the block can affect anyone else
                    if live_in.get(&bid) != Some(&live) {
                        live_in.insert(bid, live);
                        changed = true;
                    }
                }
            }

            // stage 7: now that liveness is known we can distill it into positions. this lets us
            // free and reuse registers when they're no longer needed, reducing frame size.
            let mut pos = vec![0; body.instructions.len()];
            let mut dies_at = vec![0; body.instructions.len()];
            let mut next = 0;
            for &bid in &order {
                for &iid in &body.blocks[bid].stream {
                    pos[iid.index()] = next;
                    dies_at[iid.index()] = next;
                    next += 1;
                }
            }
            // now we check for the reasons a value may still be needed
            for &bid in &order {
                let block = &body.blocks[bid];
                // reason 1: something still will read it
                for &iid in &block.stream {
                    if matches!(body.instructions[iid], Inst::Phi(_)) {
                        continue;
                    }
                    for u in uses(&body.instructions[iid]) {
                        dies_at[u.index()] = dies_at[u.index()].max(pos[iid.index()]);
                    }
                }
                let Some(&last) = block.stream.last() else {
                    continue;
                };
                // reason 2: a successor needs it on arrival, so it has to make it to the end of
                // this block. this is what makes the dependence of A/B in loop bodies described
                // above function fine
                //
                // reason 3: this block's phi copies read it at the tail.
                let (fallthrough, branches) = successors(bid);
                let flows = fallthrough
                    .into_iter()
                    .chain(branches)
                    .flat_map(|succ| live_in.get(&succ).into_iter().flatten().copied());
                let phi_reads = phi_copies.get(&bid).into_iter().flatten().map(|(_, v)| *v);
                for u in flows.chain(phi_reads) {
                    dies_at[u.index()] = dies_at[u.index()].max(pos[last.index()]);
                }
            }
            let mut dying: Vec<Vec<InstId>> = vec![Vec::new(); next];
            for &bid in &order {
                for &iid in &body.blocks[bid].stream {
                    dying[dies_at[iid.index()]].push(iid);
                }
            }

            // stage 8: swap the flagged reads from GetLocal to CopyLocal, in place. from here on
            // they're two different instructions -- a GetLocal never emits anything, a CopyLocal
            // always emits one Move into a register of its own. this has to happen after liveness
            // (which needs to see them as GetLocals to match them against a SetLocal) and before
            // emit. it's also the first write to `body` since clean, so `successors` is done for.
            for iid in needs_copy {
                if let Inst::GetLocal(l) = body.instructions[iid] {
                    body.instructions[iid] = Inst::CopyLocal(l);
                }
            }

            let non_empty: Vec<BlockId> = order
                .iter()
                .copied()
                .filter(|&id| !body.blocks[id].stream.is_empty())
                .collect();
            let fallthrough: HashMap<BlockId, BlockId> =
                non_empty.windows(2).map(|w| (w[0], w[1])).collect();

            // stage 9: emit. a few helpers first.
            //
            // `owns`: whether a value has a register of its own to give back. a GetLocal is sitting
            // in the local's register, and a phi's register is written from other blocks and never
            // reused. everything else (CopyLocal included) owns its register.
            let owns = |u: InstId| -> bool {
                !matches!(body.instructions[u], Inst::GetLocal(_) | Inst::Phi(_))
            };
            // `clean_is_safe`: iid is the very last thing that needs u, and u owns its register.
            // the shortcuts below ask this before taking over u's register or throwing its op
            // away. it used to mean "last read within this block, and never leaves it".
            let clean_is_safe = |iid: InstId, u: InstId| -> bool {
                dies_at[u.index()] == pos[iid.index()] && owns(u)
            };

            // the free list. `Ctx::reg` pops from here and only makes a new register when it's
            // empty. this used to live inside the block loop, so every block started from an empty
            // list and nothing freed in one block could be reused in the next. now that we know
            // exactly when each value dies it lives for the whole body, which is where most of the
            // frame size savings come from (eval went from 22 registers to 13). in
            // `if c { x + 100 } else { x * 2 }` the add's temp is freed after its phi copy and the
            // multiply in the else block picks it right back up.
            let mut free: Vec<Reg> = Vec::new();
            for &bid in &order {
                let block = &body.blocks[bid];

                block_offset[bid] = byte_offset;

                // prologue: materialize the cached constants once at the top of the entry block.
                // entry dominates every block, so this is live at every aliased use; runs once per
                // call. sorted by reg for deterministic bytecode (HashMap order is randomized).
                if bid == BlockId::ZERO {
                    let mut loads: Vec<(Reg, Constant)> = constants
                        .iter()
                        .map(|(c, &r)| (r, c.to_constant()))
                        .collect();
                    loads.sort_by_key(|(r, _)| r.index());
                    for (dst, constant) in loads {
                        let op = Op::LoadConst { dst, constant };
                        push_loc(byte_offset, Location::SYNTHETIC, &mut locs);
                        byte_offset += op.encoded_len();
                        body_ops.push(op);
                    }
                }

                for iid in block.stream.iter().copied() {
                    let inst = &body.instructions[iid];
                    let loc = body.locs[iid];

                    // release: everything whose dies_at was the previous position gives its
                    // register back. previous rather than current so that an instruction never
                    // lands its result on one of its own operands (the old code picked the result
                    // register first and freed operands after, same effect). this runs ahead of the
                    // shortcuts below, which `continue` past the rest.
                    //
                    // locals keep their registers (they're 0..locals.len()), which also covers a
                    // SetLocal's own "value" and a retargeted value that now lives in a local. so
                    // do pinned constants. the debug_assert is a tripwire for a double free, which
                    // would hand one register to two values and produce a wrong answer, no crash.
                    if let Some(prev) = pos[iid.index()].checked_sub(1) {
                        for &dead in &dying[prev] {
                            if let Some(reg) = inst_to_reg[dead]
                                && owns(dead)
                                && reg.index() >= body.locals.len()
                                && !constants.values().any(|&r| r == reg)
                            {
                                debug_assert!(!free.contains(&reg), "freed {reg:?} twice");
                                free.push(reg);
                                // the dump notes it on the op that was the last to need it
                                if self.disasm
                                    && !hidden.contains(&dead)
                                    && let Some(i) = body_ops.len().checked_sub(1)
                                {
                                    freed_after.entry(i).or_default().push(reg);
                                }
                            }
                        }
                    }

                    // some insts have optimization shortcuts
                    match inst {
                        Inst::Phi(_) => continue,
                        Inst::GetLocal(l) => {
                            // alias it straight away -- no need for a move. the reads where that
                            // isn't safe became CopyLocals in stage 8 and take the normal path
                            inst_to_reg[iid] = Some(local_to_reg[l]);
                            continue;
                        }
                        Inst::SetLocal(l, v) => {
                            let dst = local_to_reg[l];
                            let src = inst_to_reg[v].unwrap();
                            if clean_is_safe(iid, *v)
                                && !constants.values().any(|&r| r == src)
                                && body_ops.last().and_then(Op::reg) == Some(src)
                            {
                                body_ops.last_mut().unwrap().set_reg(dst);
                                inst_to_reg[v] = Some(dst);
                                free.push(src);
                                continue;
                            }
                        }
                        Inst::JumpIfFalse { condition, target } => {
                            let con_reg = inst_to_reg[condition].unwrap();
                            let block_target = BlockTarget::Block(*target);
                            if clean_is_safe(iid, *condition)
                                && body_ops.last().is_some_and(|op| {
                                    Op::reg(op) == Some(con_reg) && Op::is_comparison(op)
                                })
                            {
                                let cmp = body_ops.pop().unwrap();
                                byte_offset -= cmp.encoded_len();
                                hidden.insert(*condition);
                                let op = fuse_branch!(cmp, block_target;
                                    IntLt => BIntLt, IntLe => BIntLe, IntGt => BIntGt,
                                    IntGe => BIntGe, IntEq => BIntEq, IntNe => BIntNe,
                                    FloatLt => BFloatLt, FloatLe => BFloatLe, FloatGt => BFloatGt,
                                    FloatGe => BFloatGe, FloatEq => BFloatEq, FloatNe => BFloatNe;
                                    IntLtImm => BIntLtImm, IntLeImm => BIntLeImm, IntGtImm => BIntGtImm,
                                    IntGeImm => BIntGeImm, IntEqImm => BIntEqImm, IntNeImm => BIntNeImm,
                                    FloatLtImm => BFloatLtImm, FloatLeImm => BFloatLeImm, FloatGtImm => BFloatGtImm,
                                    FloatGeImm => BFloatGeImm, FloatEqImm => BFloatEqImm, FloatNeImm => BFloatNeImm
                                );

                                byte_offset += op.encoded_len();
                                body_ops.push(op);
                                continue;
                            }
                        }
                        Inst::Jump { target, .. } => {
                            // the phi copies from stage 3. these are the reads liveness counts at
                            // the tail of the predecessor
                            if let Some(copies) = phi_copies.get(&bid) {
                                for &(dst, value) in copies {
                                    let op = Op::Move {
                                        dst,
                                        src: inst_to_reg[value].unwrap(),
                                    };
                                    // phi-copy moves sit at the predecessor's tail -- borrow the
                                    // jump's loc so a fault here attributes to the same source
                                    // range.
                                    push_loc(byte_offset, loc, &mut locs);
                                    byte_offset += op.encoded_len();
                                    body_ops.push(op);
                                }
                            }

                            if fallthrough.get(&bid) == Some(target) {
                                // this is a no-op, skip it!
                                continue;
                            }
                        }
                        Inst::Constant(_) if absorbed.contains(&iid) => continue,
                        Inst::Constant(con)
                            if let Some(reg) = con.cached().and_then(|v| constants.get(&v)) =>
                        {
                            inst_to_reg[iid] = Some(*reg);
                            continue;
                        }
                        _ => {}
                    };

                    // fuse an int arith/comparison op with a constant operand into its immediate
                    // form, inlining the value instead of reading it from a register.
                    if let Some((non_const, _con, val, op, kind)) = imm_binop(body, inst) {
                        let (left, dst) = {
                            let mut ctx = Ctx {
                                inst_to_reg: &inst_to_reg,
                                local_to_reg: &local_to_reg,
                                regs: &mut regs,
                                free: &mut free,
                            };
                            (ctx.i2r(&non_const), ctx.reg())
                        };
                        use OperandKind::{Float, Int};
                        let imm = match (kind, op) {
                            (Int, BinOp::Add) => Op::AddIntImm { dst, left, val },
                            (Int, BinOp::Sub) => Op::SubIntImm { dst, left, val },
                            (Int, BinOp::Mult) => Op::MultIntImm { dst, left, val },
                            (Int, BinOp::Mod) => Op::ModIntImm { dst, left, val },
                            (Int, BinOp::LessThan) => Op::IntLtImm { dst, left, val },
                            (Int, BinOp::LessEqual) => Op::IntLeImm { dst, left, val },
                            (Int, BinOp::GreaterThan) => Op::IntGtImm { dst, left, val },
                            (Int, BinOp::GreaterEqual) => Op::IntGeImm { dst, left, val },
                            (Int, BinOp::Identity) => Op::IntEqImm { dst, left, val },
                            (Int, BinOp::NotEqual) => Op::IntNeImm { dst, left, val },
                            (Float, BinOp::Add) => Op::AddFloatImm { dst, left, val },
                            (Float, BinOp::Sub) => Op::SubFloatImm { dst, left, val },
                            (Float, BinOp::Mult) => Op::MultFloatImm { dst, left, val },
                            (Float, BinOp::Mod) => Op::ModFloatImm { dst, left, val },
                            (Float, BinOp::LessThan) => Op::FloatLtImm { dst, left, val },
                            (Float, BinOp::LessEqual) => Op::FloatLeImm { dst, left, val },
                            (Float, BinOp::GreaterThan) => Op::FloatGtImm { dst, left, val },
                            (Float, BinOp::GreaterEqual) => Op::FloatGeImm { dst, left, val },
                            (Float, BinOp::Identity) => Op::FloatEqImm { dst, left, val },
                            (Float, BinOp::NotEqual) => Op::FloatNeImm { dst, left, val },
                            _ => unreachable!(
                                "imm_binop only returns int/float arith + comparison ops"
                            ),
                        };
                        inst_to_reg[iid] = Some(dst);
                        push_loc(byte_offset, loc, &mut locs);
                        byte_offset += imm.encoded_len();
                        body_ops.push(imm);
                        continue;
                    }

                    let op = {
                        let ctx = Ctx {
                            inst_to_reg: &inst_to_reg,
                            local_to_reg: &local_to_reg,
                            regs: &mut regs,
                            free: &mut free,
                        };

                        Op::from_inst(inst, ctx)
                    };

                    inst_to_reg[iid] = op.reg();

                    push_loc(byte_offset, loc, &mut locs);
                    byte_offset += op.encoded_len();
                    body_ops.push(op);
                }
            }

            for op in &mut body_ops {
                if let Op::Jump { target }
                | Op::JumpIf { target, .. }
                | Op::BIntLt { target, .. }
                | Op::BIntLe { target, .. }
                | Op::BIntGt { target, .. }
                | Op::BIntGe { target, .. }
                | Op::BIntEq { target, .. }
                | Op::BIntNe { target, .. }
                | Op::BIntLtImm { target, .. }
                | Op::BIntLeImm { target, .. }
                | Op::BIntGtImm { target, .. }
                | Op::BIntGeImm { target, .. }
                | Op::BIntEqImm { target, .. }
                | Op::BIntNeImm { target, .. }
                | Op::BFloatLt { target, .. }
                | Op::BFloatLe { target, .. }
                | Op::BFloatGt { target, .. }
                | Op::BFloatGe { target, .. }
                | Op::BFloatEq { target, .. }
                | Op::BFloatNe { target, .. }
                | Op::BFloatLtImm { target, .. }
                | Op::BFloatLeImm { target, .. }
                | Op::BFloatGtImm { target, .. }
                | Op::BFloatGeImm { target, .. }
                | Op::BFloatEqImm { target, .. }
                | Op::BFloatNeImm { target, .. }
                | Op::ForNext { target, .. } = op
                    && let BlockTarget::Block(b) = target
                {
                    *target = BlockTarget::ByteOffset(block_offset[*b]);
                }
                if let Op::Switch { default, table, .. } = op {
                    for t in std::iter::once(default).chain(table.iter_mut()) {
                        if let BlockTarget::Block(b) = t {
                            *t = BlockTarget::ByteOffset(block_offset[*b]);
                        }
                    }
                }
            }

            let params = body
                .params
                .iter()
                .map(|local| local_to_reg[*local])
                .collect();
            let captures = body
                .captures
                .iter()
                .map(|local| local_to_reg[*local])
                .collect();
            // the latest binding of a shadowed name wins
            let locals = body
                .locals
                .iter()
                .map(|(local, _)| (body.artifacts[&local].clone(), local_to_reg[local]))
                .collect();
            let args = u16::try_from(body.params.len()).unwrap();
            let regs = u16::try_from(regs.len()).unwrap();

            if self.disasm {
                let name = names.get(&body_id).map(String::as_str).unwrap_or("<anon>");
                let moves = body_ops
                    .iter()
                    .filter(|op| matches!(op, Op::Move { .. }))
                    .count();
                println!(
                    "\n── body @{} {name:?}  ops {} · regs {regs} · moves {moves} ──",
                    body_id.index(),
                    body_ops.len(),
                );
                let mut rel = 0;
                let mut shown_line: Option<usize> = None;
                for (i, op) in body_ops.iter().enumerate() {
                    // print the source line above the ops it lowered from, when it changes.
                    let loc = locs
                        .partition_point(|(b, _)| (*b as usize) <= rel)
                        .checked_sub(1)
                        .map(|i| locs[i].1);
                    if let Some(loc) = loc
                        && !loc.is_synthetic()
                        && let Some(src) = self.srcs.get(&loc.file_id)
                    {
                        let upto = loc.span.start.min(src.len());
                        let line = src[..upto].bytes().filter(|&b| b == b'\n').count();
                        if shown_line != Some(line) {
                            shown_line = Some(line);
                            let text = src.lines().nth(line).unwrap_or("").trim();
                            println!("  {}", format!("{} │ {text}", line + 1).dimmed());
                        }
                    }
                    // byte-offset column, colored like the existing address column so jump
                    // targets are cross-referenceable.
                    let addr = format!("{:04}", chunk_offset + rel).bright_black().bold();
                    // the registers this op was the last to need
                    let freed = freed_after.get(&i).map_or(String::new(), |regs| {
                        let regs: Vec<String> =
                            regs.iter().map(|r| format!("r{}", r.index())).collect();
                        format!("· {} free", regs.join(" ")).dimmed().to_string()
                    });
                    println!("  {addr}  {op}{freed}");
                    rel += op.encoded_len();
                }
            }

            self.chunks.push(Chunk {
                offset: chunk_offset,
                args,
                regs,
                params,
                captures,
                locals,
                locs,
            });

            for op in body_ops {
                op.encode(&mut self.bytes);
            }
        }

        Program {
            // the body top-level code lowered into (`BodyId::ZERO`, or a repl's latest `new_entry`)
            entry: ir.current_body,
            root,
            chunks: self.chunks.clone(),
            signatures: self.signatures.clone(),
            strs: ir.str_interner.clone(),
            bytes: self.bytes.clone().finish(),
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct Ctx<'a> {
    pub inst_to_reg: &'a IdVec<InstId, Option<Reg>>,
    pub local_to_reg: &'a IdVec<Local, Reg>,
    pub regs: &'a mut IdVec<Reg, ()>,
    pub free: &'a mut Vec<Reg>,
}

impl Ctx<'_> {
    pub fn reg(&mut self) -> Reg {
        self.free.pop().unwrap_or_else(|| self.regs.push(()))
    }

    pub fn i2r(&self, iid: &InstId) -> Reg {
        self.inst_to_reg[iid].unwrap()
    }
}

// if `inst` is an int arith or comparison op with a constant-int operand, return (non-constant
// operand, the constant operand, value, effective op). The constant is canonicalized to the right:
// Sub/Mod only fuse a right constant; Add/Mult take either side; comparisons take either side and
// flip the operator when the constant was on the left (`2 < n` == `n > 2`).
fn imm_binop(body: &Body, inst: &Inst) -> Option<(InstId, InstId, i64, BinOp, OperandKind)> {
    use BinOp::*;
    let Inst::BinOp {
        left,
        op,
        right,
        kind,
    } = inst
    else {
        return None;
    };
    let kind = *kind;
    // int constants pass their value through; float constants are bit-cast into the i64 slot
    let cst = |i: InstId| match (kind, &body.instructions[i]) {
        (OperandKind::Int, Inst::Constant(Constant::Int(v))) => Some(*v),
        (OperandKind::Float, Inst::Constant(Constant::Float(f))) => Some(f.to_bits() as i64),
        _ => None,
    };
    let (non_const, con, val, op) = match op {
        Add | Mult => match (cst(*left), cst(*right)) {
            (_, Some(v)) => (*left, *right, v, *op),
            (Some(v), _) => (*right, *left, v, *op),
            _ => return None,
        },
        Sub | Mod => (*left, *right, cst(*right)?, *op),
        LessThan | LessEqual | GreaterThan | GreaterEqual | Identity | NotEqual => {
            match (cst(*left), cst(*right)) {
                (_, Some(v)) => (*left, *right, v, *op),
                (Some(v), _) => {
                    // gotta flip the operator
                    let op = match op {
                        BinOp::LessThan => BinOp::GreaterThan,
                        BinOp::LessEqual => BinOp::GreaterEqual,
                        BinOp::GreaterThan => BinOp::LessThan,
                        BinOp::GreaterEqual => BinOp::LessEqual,
                        other => *other, // equality is symetric
                    };
                    (*right, *left, v, op)
                }
                _ => return None,
            }
        }
        _ => return None,
    };
    Some((non_const, con, val, op, kind))
}
