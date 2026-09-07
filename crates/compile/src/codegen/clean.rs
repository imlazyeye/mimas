use crate::{
    BlockId, Constant, Inst, InstId, Scalar,
    ir::{Body, FormatPart},
};
use std::collections::{HashMap, HashSet};

/// Optimizes a body of instructions. Runs until it provides no new optimizations or the maximum
/// cycles are hit.
pub(crate) fn clean(body: &mut Body) {
    const MAXIMUM_PASSES: usize = 16;
    let mut passes = 0;
    while passes < MAXIMUM_PASSES {
        let mut modified = thread_jumps(body);
        modified |= dce(body);
        modified |= thread_phis(body);
        modified |= dce(body);
        modified |= sink_returns(body);
        modified |= dce(body);
        modified |= fold_constants(body);
        modified |= dve(body);

        if !modified {
            break;
        }

        passes += 1;
    }

    if passes == MAXIMUM_PASSES {
        // TODO: emit this more elegantly
        eprintln!("WARNING: hit the maximum number of clean passes, continuing...");
    }
}

/// Prunes dead code. This is effectively really bad pathfinding -- we use the same kind of queue
/// pattern, but we're gonna check EVERYTHING!
fn dce(body: &mut Body) -> bool {
    let mut seen = HashSet::new();
    let mut work = vec![BlockId::ZERO];
    let mut modified = false;
    while let Some(b) = work.pop() {
        if !seen.insert(b) {
            continue;
        }

        for &iid in &body.blocks[b].stream {
            match &body.instructions[iid] {
                Inst::Jump { target }
                | Inst::JumpIfFalse { target, .. }
                | Inst::ForNext { target, .. } => work.push(*target),
                Inst::Switch { table, default, .. } => {
                    work.extend(table.iter().copied());
                    work.push(*default);
                }
                _ => {}
            }
        }
    }

    // an unreachable block stays in `blocks` with an emptied stream, so only count a clear that
    // actually removed something -- otherwise we'd report "modified" every pass and never converge.
    for (bid, block) in body.blocks.iter_mut() {
        if !seen.contains(&bid) && !block.stream.is_empty() {
            block.stream.clear();
            modified = true;
        }
    }

    modified
}

/// Prunes unused values. Values must not be roots as defined in [Inst::is_root].
fn dve(body: &mut Body) -> bool {
    let mut live = HashSet::new();
    let mut work = Vec::new();

    for (_, block) in body.blocks.iter() {
        for &iid in &block.stream {
            if body.instructions[iid].is_root() {
                work.push(iid);
            }
        }
    }

    while let Some(iid) = work.pop() {
        if !live.insert(iid) {
            continue;
        }

        for u in uses(&body.instructions[iid]) {
            work.push(u);
        }
    }

    let mut modified = false;
    for (_, block) in body.blocks.iter_mut() {
        let before = block.stream.len();
        block.stream.retain(|&iid| live.contains(&iid));
        modified |= block.stream.len() != before;
    }

    modified
}

/// Yields the InstIds that an Inst relies upon.
pub(crate) fn uses(inst: &Inst) -> Vec<InstId> {
    let mut out = match inst {
        Inst::SetLocal(_, v) => vec![*v],
        Inst::BinOp { left, right, .. } => vec![*left, *right],
        Inst::UnaryOp { right, .. } => vec![*right],
        Inst::JumpIfFalse { condition, .. } => vec![*condition],
        Inst::SetIndex { set, index, value } => vec![*set, *index, *value],
        Inst::GetIndex { set, index, .. } => vec![*set, *index],
        Inst::GetField { src, .. } => vec![*src],
        Inst::SetField {
            receiver, value, ..
        } => vec![*receiver, *value],
        Inst::Push { array, value } => vec![*array, *value],
        Inst::Insert { value, .. } => vec![*value],
        Inst::Len(v)
        | Inst::ToFloat(v)
        | Inst::Sqrt(v)
        | Inst::Unwrap(v)
        | Inst::Return(v)
        | Inst::Raise(v)
        | Inst::IsRaised(v)
        | Inst::UnwrapRaised(v) => vec![*v],
        Inst::In(a, b, _) => vec![*a, *b],
        Inst::ForNext { idx, bound, .. } => vec![*idx, *bound],
        Inst::IsInstance { src, .. } => vec![*src],
        Inst::Switch { scrut, .. } => vec![*scrut],
        Inst::Format(parts) => parts
            .iter()
            .filter_map(|p| {
                if let FormatPart::Value(v) = p {
                    Some(*v)
                } else {
                    None
                }
            })
            .collect(),
        Inst::MakeClosure { captures, .. } => captures.clone(),
        Inst::Call { callee, args } => std::iter::once(*callee)
            .chain(args.iter().copied())
            .collect(),
        Inst::CallDirect { args, .. } | Inst::CallNative { args, .. } => args.clone(),
        Inst::NewInstance { fields, .. } => fields.clone(),
        Inst::Phi(branches) => branches.iter().map(|(_, v)| *v).collect(),
        _ => vec![],
    };

    out.sort_by_key(|i| i.index());
    out.dedup();
    out
}

/// Yields the operands that die in this block.
pub(crate) fn last_uses(body: &Body, block: BlockId) -> HashMap<InstId, InstId> {
    let mut last_use = HashMap::new();
    for &iid in body.blocks[block].stream.iter().rev() {
        for u in uses(&body.instructions[iid]) {
            last_use.entry(u).or_insert(iid);
        }
    }
    last_use
}

// any inst used in a block other than the one it was defined in, so they don't get freed.
pub(crate) fn cross_block_iids(body: &Body) -> HashSet<InstId> {
    let mut defs = HashMap::new();
    for (bid, block) in body.blocks.iter() {
        for &iid in &block.stream {
            defs.insert(iid, bid);
        }
    }
    let mut cross = HashSet::new();
    for (bid, block) in body.blocks.iter() {
        for &iid in &block.stream {
            for u in uses(&body.instructions[iid]) {
                if defs.get(&u) != Some(&bid) {
                    cross.insert(u);
                }
            }
        }
    }
    cross
}

/// Finds blocks that are exclusively a jump and remaps them so that they can be skipped.
///
/// Example: given b0, b1, and b2, if b0 jumps to b1, and b1 has only one instruction which is to
/// jump to b2, b0 can instead just go directly to b2.
fn thread_jumps(body: &mut Body) -> bool {
    let mut orphans = HashMap::new();
    for (bid, block) in body.blocks.iter() {
        if let [orphan] = block.stream[..]
            && let Inst::Jump { target } = body.instructions[orphan]
        {
            orphans.insert(bid, target);
        }
    }

    let mut redirections = HashMap::new();
    for bid in orphans.keys() {
        let mut target = bid;

        // the natural way to write this would be `while let Some(_) = orphans.get(target`, however
        // that runs the risk of running forever if we form a cycle. instead, we check at a maximum
        // the number of blocks that there actually are, otherwise we know we're in a cycle.
        for _ in 0..body.blocks.len() {
            match orphans.get(target) {
                Some(n) => target = n,
                None => break,
            }
        }

        // if we found a redireciton and it _doesn't_ have a phi in it, we can redirect this. we
        // leave things with phi alone because otherwise we'd risk desynching the phi's target.
        if target != bid
            && !body.blocks[target]
                .stream
                .first()
                .is_some_and(|&i| matches!(body.instructions[i], Inst::Phi(_)))
        {
            redirections.insert(bid, target);
        }
    }

    // now we can redirect them! no more orphans. we're heros! redirections only ever point at
    // non-phi blocks (filtered above), so a switch edge can be redirected too without landing on a
    // phi block it couldn't feed a phi-copy into.
    // report whether a target was actually rewritten -- a forwarder reached only by fall-through
    // produces a redirection that never lands, and counting that would spin the fixpoint forever.
    let mut modified = false;
    for (_, inst) in body.instructions.iter_mut() {
        match inst {
            Inst::Jump { target }
            | Inst::JumpIfFalse { target, .. }
            | Inst::ForNext { target, .. } => {
                if let Some(&redirect) = redirections.get(target) {
                    *target = *redirect;
                    modified = true;
                }
            }
            Inst::Switch { table, default, .. } => {
                for target in table.iter_mut().chain(std::iter::once(default)) {
                    if let Some(&redirect) = redirections.get(target) {
                        *target = *redirect;
                        modified = true;
                    }
                }
            }
            _ => {}
        }
    }

    modified
}

fn thread_phis(body: &mut Body) -> bool {
    // first we have to find predecessors. these are the places that point to the jumps that we
    // might remap, so that if we're going to remove jump T from a phi's branches, we insert all of
    // the ways you can get to T to phi's branches, as we may be expanding the possible routes of
    // entry.
    let mut preds: HashMap<_, Vec<_>> = HashMap::new();
    for (bid, block) in body.blocks.iter() {
        for &iid in &block.stream {
            if let Some(target) = body.instructions[iid].jump_target() {
                preds.entry(target).or_default().push(bid);
            }
        }
    }

    // a switch edge can't host the phi-copy a Jump edge does (codegen emits copies only before a
    // Jump), so a forwarder a switch points at must stay put to host it -- never fold it away.
    let switch_targets: HashSet<BlockId> = body
        .blocks
        .iter()
        .flat_map(|(_, block)| block.stream.iter())
        .filter_map(|&iid| match &body.instructions[iid] {
            Inst::Switch { table, default, .. } => {
                Some(table.iter().copied().chain(std::iter::once(*default)))
            }
            _ => None,
        })
        .flatten()
        .collect();

    // plan the rewrites: (forwarder, its phi-block target, the forwarder's preds).
    let mut plan: Vec<(_, _, Vec<_>)> = Vec::new();
    for (bid, _) in body.blocks.iter() {
        // Only fold forwarders...
        let Some(t) = forwarder_target(body, bid) else {
            continue;
        };
        if switch_targets.contains(&bid) {
            continue;
        }

        // ...whose target opens with a phi -- that's what we splice into.
        let target_has_phi = body.blocks[t]
            .stream
            .first()
            .is_some_and(|&i| matches!(body.instructions[i], Inst::Phi(_)));
        if !target_has_phi {
            continue;
        }

        let ps = preds.get(&bid).cloned().unwrap_or_default();

        // skip if any predecessor already jumps to T (would give T two phi entries from the
        // same predecessor -- malformed), is itself a forwarder (avoids chain-ordering hazards
        // within this single pass), or reaches the forwarder via a conditional jump. that last
        // case is load-bearing: the forwarder IS the critical-edge split, and codegen only
        // materializes a phi-copy before an unconditional Jump -- redirect the JumpIfFalse
        // straight into the phi block and the taken branch skips the copy entirely.
        let dangerous_target = ps.iter().any(|&p| {
            body.blocks[p].stream.iter().any(|&i| {
                body.instructions[i].jump_target() == Some(t)
                    || matches!(body.instructions[i], Inst::JumpIfFalse { target, .. } | Inst::ForNext { target, .. } if target == bid)
            }) || forwarder_target(body, p).is_some()
        });
        if dangerous_target {
            continue;
        }

        plan.push((bid, t, ps));
    }

    // now do the thing!
    for (bid, targ, preds) in &plan {
        // replace the forwarder's phi entry with one entry per real predecessor
        for &i in &body.blocks[targ].stream {
            if let Inst::Phi(branches) = &mut body.instructions[i]
                && let Some(pos) = branches.iter().position(|&(blk, _)| blk == *bid)
            {
                let (_, value) = branches.remove(pos);
                for &p in preds {
                    branches.push((p, value));
                }
            }
        }

        // redirect each predecessor's edge from bid straight to targ.
        for &p in preds {
            for &i in &body.blocks[p].stream {
                if let Some(target) = body.instructions[i].jump_target_mut()
                    && *target == *bid
                {
                    *target = *targ;
                }
            }
        }
    }

    !plan.is_empty()
}

/// Folds constants that we know at this point (both user constants and literals, i.e.: NUM + 1).
/// We only bother to check floats/ints right now because those are by far the most plausible to be
/// hot paths, though we could also check for bools and others in the future.
fn fold_constants(body: &mut Body) -> bool {
    let mut folds: Vec<(InstId, Constant)> = Vec::new();
    for (iid, inst) in body.instructions.iter() {
        let Inst::BinOp {
            left, op, right, ..
        } = inst
        else {
            continue;
        };
        let (Inst::Constant(l), Inst::Constant(r)) =
            (&body.instructions[*left], &body.instructions[*right])
        else {
            continue;
        };
        let result = match (l, r) {
            (Constant::Int(a), Constant::Int(b)) => op.eval_int(*a, *b),
            (Constant::Float(a), Constant::Float(b)) => op.eval_float(*a, *b),
            (Constant::Int(a), Constant::Float(b)) => op.eval_float(*a as f64, *b),
            (Constant::Float(a), Constant::Int(b)) => op.eval_float(*a, *b as f64),
            _ => continue,
        };
        if let Ok(scalar) = result {
            let folded = match scalar {
                Scalar::Int(i) => Constant::Int(i),
                Scalar::Bool(b) => Constant::Bool(b),
                Scalar::Float(f) => Constant::Float(f),
            };
            folds.push((iid, folded));
        }
    }

    let modified = !folds.is_empty();
    for (iid, folded) in folds {
        body.instructions[iid] = Inst::Constant(folded);
    }
    modified
}

fn sink_returns(body: &mut Body) -> bool {
    let mut merges: Vec<(BlockId, InstId)> = Vec::new();
    for (bid, block) in body.blocks.iter() {
        if let [phi_iid, jump_iid] = block.stream[..]
            && matches!(body.instructions[phi_iid], Inst::Phi(_))
            && let Inst::Jump { target: ret_block } = body.instructions[jump_iid]
            && let [ret_iid] = body.blocks[ret_block].stream[..]
            && matches!(body.instructions[ret_iid], Inst::Return(v) if v == phi_iid)
        {
            merges.push((bid, phi_iid));
        }
    }

    let mut preds: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for (bid, block) in body.blocks.iter() {
        for &iid in &block.stream {
            if let Some(t) = body.instructions[iid].jump_target() {
                preds.entry(t).or_default().push(bid);
            }
        }
    }

    let mut modified = false;
    for (merge, phi_iid) in merges {
        for p in preds.get(&merge).cloned().unwrap_or_default() {
            let edges: Vec<InstId> = body.blocks[p]
                .stream
                .iter()
                .copied()
                .filter(|&iid| body.instructions[iid].jump_target().is_some())
                .collect();
            let [edge] = edges[..] else { continue };
            if !matches!(body.instructions[edge], Inst::Jump { target } if target == merge) {
                continue;
            }

            let Inst::Phi(branches) = &mut body.instructions[phi_iid] else {
                unreachable!()
            };
            let Some(pos) = branches.iter().position(|&(blk, _)| blk == p) else {
                continue;
            };
            let v_p = branches[pos].1;
            branches.remove(pos);

            body.instructions[edge] = Inst::Return(v_p);
            modified = true;
        }
    }
    modified
}

/// If bid is a forwarder -- exactly one unconditional Jump -- return its target.
fn forwarder_target(body: &Body, bid: BlockId) -> Option<BlockId> {
    match body.blocks[bid].stream[..] {
        [only] => match body.instructions[only] {
            Inst::Jump { target } => Some(target),
            _ => None,
        },
        _ => None,
    }
}
