use shared::IdVec;

use colored::Colorize;
use shared::{FileId, Location};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    BinOp, BlockId, BlockTarget, BodyId, Constant, Inst, InstId, Ir, Local, Op, OperandKind,
    codegen::{
        clean::{self, uses},
        program::{Chunk, Encoder, Program},
    },
    ir::Body,
};

shared::id!(pub Reg);

#[derive(Debug)]
pub struct Compiler {
    pub(crate) ops: Vec<Op>,
    pub(crate) chunks: IdVec<BodyId, Chunk>,
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
            ops: Vec::new(),
            chunks: IdVec::new(),
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

    pub fn compile(&mut self, ir: Ir) -> Program {
        let mut bytes = Encoder::new();

        // body -> source name, for the --disasm dump only
        let names: HashMap<BodyId, String> = if self.disasm {
            ir.item_bodies
                .iter()
                .map(|(&dec, &body)| (body, ir.resolutions.decs[dec].name.clone()))
                .collect()
        } else {
            HashMap::new()
        };

        for (body_id, mut body) in ir.bodies {
            clean::clean(&mut body);

            let mut regs: IdVec<Reg, ()> = IdVec::new();
            let mut local_to_reg: IdVec<Local, Reg> = IdVec::new();
            let cross = clean::cross_block_iids(&body);
            let last_use_by_block: HashMap<BlockId, HashMap<InstId, InstId>> = body
                .blocks
                .iter()
                .map(|(bid, _)| (bid, clean::last_uses(&body, bid)))
                .collect();

            body.locals.iter().for_each(|_| {
                local_to_reg.push(regs.push(()));
            });

            let mut inst_to_reg: IdVec<InstId, Option<Reg>> =
                vec![None; body.instructions.len()].into();

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

            let chunk_offset = bytes.len();
            let mut byte_offset = chunk_offset;
            let mut body_ops = Vec::new();
            let mut block_offset: IdVec<BlockId, usize> =
                IdVec::from(vec![0usize; body.blocks.len()]);
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
                        let absorbs = imm_binop(&body, inst).map(|(_, con, _, _, _)| con);
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

            // pre-scan: give each DISTINCT cache-friendly constant a pinned register, so
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

            // successors of a block: (fall-through, branches)
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

            let non_empty: Vec<BlockId> = order
                .iter()
                .copied()
                .filter(|&id| !body.blocks[id].stream.is_empty())
                .collect();
            let fallthrough: HashMap<BlockId, BlockId> =
                non_empty.windows(2).map(|w| (w[0], w[1])).collect();

            for &bid in &order {
                let block = &body.blocks[bid];
                let last_use = &last_use_by_block[&bid];
                let mut free: Vec<Reg> = Vec::new();

                let clean_is_safe = |iid: InstId, u: InstId| -> bool {
                    last_use.get(&u) == Some(&iid)
                        && !cross.contains(&u)
                        && !matches!(body.instructions[u], Inst::GetLocal(_) | Inst::Phi(_))
                };

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

                    // some insts have optimization shortcuts
                    match inst {
                        Inst::Phi(_) => continue,
                        Inst::GetLocal(l) => {
                            // alias it straight away -- no need for a move
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
                    if let Some((non_const, _con, val, op, kind)) = imm_binop(&body, inst) {
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
                        if clean_is_safe(iid, non_const) && !constants.values().any(|&r| r == left)
                        {
                            free.push(left);
                        }
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

                    // free each operand whose last use is here (uses() is already deduped, so a
                    // value filling two slots like `x * x` won't get freed twice). a pinned
                    // constant register is never recycled -- its value must outlive every use.
                    for u in uses(&body.instructions[iid]) {
                        let reg = inst_to_reg[u].unwrap();
                        if clean_is_safe(iid, u) && !constants.values().any(|&r| r == reg) {
                            free.push(reg);
                        }
                    }

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
            let locals = body
                .artifacts
                .iter()
                .map(|(local, name)| (name.clone(), local_to_reg[*local]))
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
                for op in body_ops.iter() {
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
                    println!("  {addr}  {op}");
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

            for op in body_ops.iter().cloned() {
                op.encode(&mut bytes);
            }
            self.ops.extend(body_ops);
        }

        let items: HashMap<String, BodyId> = ir
            .item_bodies
            .iter()
            .map(|(&dec, &body)| (ir.resolutions.decs[dec].name.clone(), body))
            .collect();

        Program {
            entry: BodyId::ZERO,
            chunks: std::mem::replace(&mut self.chunks, IdVec::new()),
            strs: ir.str_interner,
            bytes: bytes.finish(),
            items,
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
