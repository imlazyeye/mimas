use std::{rc::Rc, sync::Arc};

use api::Registry;
use compile::{
    AccessKind, BinOp, BodyId, Chunk, Constant, Decode, Decoder, Function, Module, OpCode,
    OpFormatPart, Program, Reg, UnaryOp,
};
use gc_arena::{Arena, Gc, Rootable};
use shared::{Error, FnHeader, IdVec, StrInterner};
use smallvec::SmallVec;

use crate::{
    Closure, Ctx, DictMap, Fields, Frame, INLINE_FIELDS, LocatedRtErr, RtErr, RtResult, Sources,
    Stashed, State, ThreadState, Val,
    conversion::{Args, MimasType},
};

const FUEL: usize = 1024;

#[cfg(feature = "op-count")]
mod op_count {
    use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

    use compile::OpCode;

    pub static COUNTS: [AtomicU64; OpCode::COUNT] = [const { AtomicU64::new(0) }; OpCode::COUNT];

    pub fn report() {
        let mut rows: Vec<(OpCode, u64)> = (0..OpCode::COUNT)
            // SAFETY: repr(u8) enum, i < COUNT, same as the decode transmute
            .map(|i| {
                (
                    unsafe { std::mem::transmute::<u8, OpCode>(i as u8) },
                    COUNTS[i].load(Relaxed),
                )
            })
            .collect();
        rows.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
        let total: u64 = rows.iter().map(|&(_, n)| n).sum();
        eprintln!("op counts ({total} total):");
        for (op, n) in rows {
            eprintln!("{n:>14}  {:>6.2}%  {op:?}", n as f64 / total as f64 * 100.0);
        }
    }
}

pub struct Vm {
    pub(crate) entry: BodyId,
    pub(crate) code: Decoder,
    pub(crate) chunks: IdVec<BodyId, Chunk>,
    pub(crate) signatures: Rc<IdVec<BodyId, Option<Function>>>,
    /// as in, strings from the compiler, not "c string". i know this is dumb and yet here I am
    pub(crate) c_strs: StrInterner,
    pub(crate) arena: Arena<Rootable![State<'_>]>,
    pub(crate) sources: Sources,
    pub(crate) root: Rc<Module>,
    pub(crate) registry: Registry,
}

impl Vm {
    pub fn new() -> Self {
        // clippy wants `Arena::new(State::new)`, but that doesn't compile -- the closure is what
        // lets inference tie `State<'_>` to `Rootable::Root<'gc>`
        #[allow(clippy::redundant_closure)]
        let arena = Arena::new(|mc| State::new(mc));
        Self {
            entry: BodyId::ZERO,
            code: Decoder {
                bytes: Vec::new(),
                ip: 0,
            },
            chunks: IdVec::new(),
            signatures: Rc::default(),
            c_strs: StrInterner::new(),
            arena,
            sources: Sources::new(),
            root: Rc::default(),
            registry: Registry::new(),
        }
    }

    /// Load `program`, replacing whatever this Vm was running.
    pub fn load_program(&mut self, program: Program) {
        let Program {
            entry,
            chunks,
            signatures,
            strs,
            bytes,
            root,
        } = program;
        self.entry = entry;
        self.code = Decoder { bytes, ip: 0 };
        self.chunks = chunks;
        self.c_strs = strs;
        self.root = Rc::new(root);
        self.install_signatures(signatures);
        let entry_chunk = &self.chunks[self.entry];
        let regs_count = entry_chunk.regs as usize;
        let entry_offset = entry_chunk.offset;
        let entry_body = self.entry;
        self.arena.mutate(|mc, state| {
            let ctx = state.ctx(mc);
            ctx.reset_roots();
            let mut t = state.thread.borrow_mut(mc);
            t.regs.clear();
            t.regs.resize(regs_count, Val::Null);
            t.frames.clear();
            t.frames.push(Frame {
                chunk: entry_body,
                ip: entry_offset,
                return_reg: 0,
                base: 0,
            });
        });
    }

    /// Install `signatures` as the loaded program's table.
    fn install_signatures(&mut self, signatures: IdVec<BodyId, Option<Function>>) {
        self.signatures = Rc::new(signatures);
        let signatures = Rc::clone(&self.signatures);
        self.arena.mutate(|mc, state| {
            *state
                .ctx(mc)
                .fixture::<crate::heap::Signatures>()
                .0
                .borrow_mut() = signatures;
        });
    }

    pub fn set_sources(&mut self, sources: Sources) {
        self.sources = sources;
    }

    /// Get a stable handle to a per-Vm fixture (e.g. a `FreezeCell`). The handle has no
    /// lifetime ties to `&self`, so it can be held across later `&mut self` calls like
    /// `run`. The caller must ensure the handle is dropped before the Vm itself. See the
    /// [fixtures](crate::fixtures) module docs for the full story.
    pub fn fixture<T: crate::fixtures::Fixture>(&self) -> FixtureRef<T> {
        let ptr: *const T = self
            .arena
            .mutate(|mc, state| state.ctx(mc).fixture::<T>() as *const T);
        FixtureRef { ptr }
    }

    pub fn run(&mut self) -> Result<(), Error> {
        loop {
            let Vm {
                code,
                chunks,
                signatures,
                c_strs: strs,
                arena,
                sources,
                ..
            } = self;
            let done = arena.mutate(|mc, state| {
                let ctx = state.ctx(mc);
                let mut thread = state.thread.borrow_mut(mc);
                run_dispatch(
                    ctx,
                    code,
                    chunks,
                    signatures,
                    strs,
                    sources,
                    &mut thread,
                    FUEL,
                    1,
                )
            })?;
            if done {
                #[cfg(feature = "op-count")]
                op_count::report();
                return Ok(());
            }
            self.arena.collect_debt();
        }
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

/// Stable reference to a Vm fixture. Lifetime-free so it doesn't conflict with
/// later `&mut Vm` borrows; the caller is responsible for not letting it outlive
/// its source `Vm`.
pub struct FixtureRef<T: 'static> {
    ptr: *const T,
}

impl<T: 'static> std::ops::Deref for FixtureRef<T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: T: 'static so it has no internal 'gc-dependent slots. The Gc
        // allocation lives for the arena, which outlives any FixtureRef under
        // the caller's discipline of dropping the handle before the Vm.
        unsafe { &*self.ptr }
    }
}

/// What `step_one` hands back to the dispatch loop: keep going, or a frame transition that has
/// to touch `thread` -- and so can only run once the register window borrow has been dropped.
enum Flow<'gc> {
    Next,
    Call {
        target: CallTarget<'gc>,
        dst: Reg,
        args: SmallVec<[Val<'gc>; 8]>,
    },
    Return(Val<'gc>),
}

enum CallTarget<'gc> {
    Fn(BodyId),
    Value(BodyId),
    Closure(Closure<'gc>),
}

/// Read a register out of the current frame's window. `$regs` is the local `&mut [Val]` slice;
/// codegen guarantees every index is `< chunk.regs == window len`, so the bounds check is dead.
macro_rules! rd {
    ($regs:expr, $r:expr) => {{
        let r = $r;
        debug_assert!(r.index() < $regs.len());
        // SAFETY: register indices are compiler-allocated in 0..chunk.regs == window len.
        unsafe { *$regs.get_unchecked(r.index()) }
    }};
}

/// Write a register in the current frame's window. Evaluates the value before taking the
/// `&mut` so `wr!(regs, d, rd!(regs, s))` stays legal.
macro_rules! wr {
    ($regs:expr, $r:expr, $v:expr) => {{
        let r = $r;
        let v = $v;
        debug_assert!(r.index() < $regs.len());
        // SAFETY: as in `rd!`.
        unsafe { *$regs.get_unchecked_mut(r.index()) = v };
    }};
}

// The fast-path macros below all share one shape: decode operands, try the in-type case inline,
// and on any other type combination `return` the matching cold helper. The `return` (not `?`) is
// load-bearing: it puts the cold call in tail position so the arm forwards the helper's
// `RtResult<Flow>` verbatim -- no per-arm Err-widening, and the operands are consumed by value by
// the cold fn and never read again here, so they never need a stack home across the tag check.
macro_rules! int_arith {
    ($regs:ident, $code:ident, $ctx:ident, $checked:ident, $op:expr) => {{
        let dst = Reg::decode($code);
        let left = Reg::decode($code);
        let right = Reg::decode($code);
        match (rd!($regs, left), rd!($regs, right)) {
            (Val::Int(a), Val::Int(b)) => {
                let Some(v) = a.$checked(b) else {
                    return Err(RtErr::IntegerOverflow);
                };
                wr!($regs, dst, Val::Int(v));
            }
            _ => return bin_cold($regs, dst, left, right, $ctx, $op),
        }
    }};
}

macro_rules! int_eval {
    ($regs:ident, $code:ident, $ctx:ident, $rust_op:tt, $op:expr) => {{
        let dst = Reg::decode($code);
        let left = Reg::decode($code);
        let right = Reg::decode($code);
        match (rd!($regs, left), rd!($regs, right)) {
            (Val::Int(a), Val::Int(b)) => wr!($regs, dst, Val::Bool(a $rust_op b)),
            _ => return bin_cold($regs, dst, left, right, $ctx, $op),
        }
    }};
}

macro_rules! float_arith {
    ($regs:ident, $code:ident, $ctx:ident, $rust_op:tt, $op:expr) => {{
        let dst = Reg::decode($code);
        let left = Reg::decode($code);
        let right = Reg::decode($code);
        match (rd!($regs, left), rd!($regs, right)) {
            (Val::Float(a), Val::Float(b)) => wr!($regs, dst, Val::Float(a $rust_op b)),
            _ => return bin_cold($regs, dst, left, right, $ctx, $op),
        }
    }};
}

macro_rules! float_eval {
    ($regs:ident, $code:ident, $ctx:ident, $rust_op:tt, $op:expr) => {{
        let dst = Reg::decode($code);
        let left = Reg::decode($code);
        let right = Reg::decode($code);
        match (rd!($regs, left), rd!($regs, right)) {
            (Val::Float(a), Val::Float(b)) => wr!($regs, dst, Val::Bool(a $rust_op b)),
            _ => return bin_cold($regs, dst, left, right, $ctx, $op),
        }
    }};
}

macro_rules! str_eval {
    ($regs:ident, $code:ident, $ctx:ident, $rust_op:tt, $op:expr) => {{
        let dst = Reg::decode($code);
        let left = Reg::decode($code);
        let right = Reg::decode($code);
        match (rd!($regs, left), rd!($regs, right)) {
            (Val::Str(a), Val::Str(b)) => wr!($regs, dst, Val::Bool(a $rust_op b)),
            _ => return bin_cold($regs, dst, left, right, $ctx, $op),
        }
    }};
}

macro_rules! branch_int {
    ($regs:ident, $code:ident, $ctx:ident, $rust_op:tt, $op:expr) => {{
        let target = $code.u32() as usize;
        let left = Reg::decode($code);
        let right = Reg::decode($code);
        let is_true = bool::decode($code);
        let hit = match (rd!($regs, left), rd!($regs, right)) {
            (Val::Int(a), Val::Int(b)) => a $rust_op b,
            _ => return branch_cold($regs, $code, target, is_true, left, right, $ctx, $op),
        };
        if hit == is_true {
            $code.ip = target;
        }
    }};
}

macro_rules! branch_float {
    ($regs:ident, $code:ident, $ctx:ident, $rust_op:tt, $op:expr) => {{
        let target = $code.u32() as usize;
        let left = Reg::decode($code);
        let right = Reg::decode($code);
        let is_true = bool::decode($code);
        let hit = match (rd!($regs, left), rd!($regs, right)) {
            (Val::Float(a), Val::Float(b)) => a $rust_op b,
            _ => return branch_cold($regs, $code, target, is_true, left, right, $ctx, $op),
        };
        if hit == is_true {
            $code.ip = target;
        }
    }};
}

macro_rules! int_arith_imm {
    ($regs:ident, $code:ident, $ctx:ident, $checked:ident, $op:expr) => {{
        let dst = Reg::decode($code);
        let left = Reg::decode($code);
        let val = $code.i64();
        match rd!($regs, left) {
            Val::Int(a) => {
                let Some(v) = a.$checked(val) else {
                    return Err(RtErr::IntegerOverflow);
                };
                wr!($regs, dst, Val::Int(v));
            }
            _ => return bin_cold_imm_int($regs, dst, left, val, $ctx, $op),
        }
    }};
}

macro_rules! int_eval_imm {
    ($regs:ident, $code:ident, $ctx:ident, $rust_op:tt, $op:expr) => {{
        let dst = Reg::decode($code);
        let left = Reg::decode($code);
        let val = $code.i64();
        match rd!($regs, left) {
            Val::Int(a) => wr!($regs, dst, Val::Bool(a $rust_op val)),
            _ => return bin_cold_imm_int($regs, dst, left, val, $ctx, $op),
        }
    }};
}

macro_rules! branch_int_imm {
    ($regs:ident, $code:ident, $ctx:ident, $rust_op:tt, $op:expr) => {{
        let target = $code.u32() as usize;
        let left = Reg::decode($code);
        let val = $code.i64();
        let is_true = bool::decode($code);
        let hit = match rd!($regs, left) {
            Val::Int(a) => a $rust_op val,
            _ => return branch_cold_imm_int($regs, $code, target, is_true, left, val, $ctx, $op),
        };
        if hit == is_true {
            $code.ip = target;
        }
    }};
}

macro_rules! float_arith_imm {
    ($regs:ident, $code:ident, $ctx:ident, $rust_op:tt, $op:expr) => {{
        let dst = Reg::decode($code);
        let left = Reg::decode($code);
        let val = f64::from_bits($code.i64() as u64);
        let v = match rd!($regs, left) {
            Val::Float(l) => Val::Float(l $rust_op val),
            _ => return bin_cold_imm_float($regs, dst, left, val, $ctx, $op),
        };
        wr!($regs, dst, v);
    }};
}

macro_rules! float_eval_imm {
    ($regs:ident, $code:ident, $ctx:ident, $rust_op:tt, $op:expr) => {{
        let dst = Reg::decode($code);
        let left = Reg::decode($code);
        let val = f64::from_bits($code.i64() as u64);
        match rd!($regs, left) {
            Val::Float(a) => wr!($regs, dst, Val::Bool(a $rust_op val)),
            _ => return bin_cold_imm_float($regs, dst, left, val, $ctx, $op),
        }
    }};
}

macro_rules! branch_float_imm {
    ($regs:ident, $code:ident, $ctx:ident, $rust_op:tt, $op:expr) => {{
        let target = $code.u32() as usize;
        let left = Reg::decode($code);
        let val = f64::from_bits($code.i64() as u64);
        let is_true = bool::decode($code);
        let hit = match rd!($regs, left) {
            Val::Float(l) => l $rust_op val,
            _ => return branch_cold_imm_float($regs, $code, target, is_true, left, val, $ctx, $op),
        };
        if hit == is_true {
            $code.ip = target;
        }
    }};
}

/// The dispatch driver. One flat loop: each op gets a fresh `noalias` `&mut [Val]` window built
/// from a raw `(ptr, len)` via `from_raw_parts_mut` -- `noalias` keeps register access fast in the
/// hot loop, and building from a raw pointer (rather than borrowing `thread.regs`) lets call/return
/// resize `thread.regs` *inline* without a borrow conflict. The window is refreshed after every
/// `thread.regs` mutation so the pointer never dangles and never overlaps another live borrow.
/// `stop_depth` is the frame floor: returning out of frame `stop_depth` ends the dispatch.
/// `Vm::run` passes 1 (the entry frame's own return is the end of the program); `Vm::call`
/// passes the pre-call depth + 1 so dispatch stops -- result written, entry ip untouched --
/// when the injected call returns, instead of running off the end of the entry's bytecode.
#[allow(clippy::too_many_arguments)]
fn run_dispatch<'gc>(
    ctx: Ctx<'gc>,
    code: &mut Decoder,
    chunks: &IdVec<BodyId, Chunk>,
    signatures: &IdVec<BodyId, Option<Function>>,
    strs: &StrInterner,
    sources: &Sources,
    thread: &mut ThreadState<'gc>,
    mut fuel: usize,
    stop_depth: usize,
) -> Result<bool, Error> {
    let (mut regs_ptr, mut regs_len) = window(thread, chunks);
    loop {
        if fuel == 0 {
            return Ok(false);
        }
        fuel -= 1;
        let op_ip = code.ip;
        #[cfg(feature = "op-count")]
        op_count::COUNTS[code.bytes[op_ip] as usize]
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // SAFETY: regs_ptr/regs_len describe the current top frame's window
        // (regs[base..base+count]), refreshed after every resize/truncate below. No op between
        // refreshes touches thread.regs, so the pointer stays valid and this is the only live
        // reference into the window.
        let regs = unsafe { std::slice::from_raw_parts_mut(regs_ptr, regs_len) };
        match step_one(regs, code, ctx, strs) {
            Ok(Flow::Next) => {}
            Ok(Flow::Call { target, dst, args }) => {
                let (body, captures): (BodyId, &[Val<'gc>]) = match &target {
                    CallTarget::Fn(b) => (*b, &[]),
                    CallTarget::Value(b) => {
                        if signatures.get(*b).and_then(Option::as_ref).is_none() {
                            let kind = not_callable(Val::Fn(*b));
                            return Err(locate(kind, op_ip, thread, chunks, sources));
                        }
                        (*b, &[])
                    }
                    CallTarget::Closure(c) => {
                        let data = Gc::as_ref(c.0);
                        (data.function, &data.captures)
                    }
                };
                if let Err(kind) = enter_call(thread, code, chunks, body, dst, &args, captures) {
                    return Err(locate(kind, op_ip, thread, chunks, sources));
                }
                (regs_ptr, regs_len) = window(thread, chunks);
            }
            Ok(Flow::Return(value)) => {
                if thread.frames.len() == 1 {
                    thread.frames.last_mut().unwrap().ip = code.ip;
                    return Ok(true);
                }
                let popped = thread.frames.pop().unwrap();
                thread.regs.truncate(popped.base);
                let caller = thread.frames.last().unwrap();
                code.ip = caller.ip;
                let caller_base = caller.base;
                thread.regs[caller_base + popped.return_reg as usize] = value;
                if thread.frames.len() < stop_depth {
                    return Ok(true);
                }
                (regs_ptr, regs_len) = window(thread, chunks);
            }
            Err(kind) => return Err(locate(kind, op_ip, thread, chunks, sources)),
        }
    }
}

/// Raw `(ptr, len)` for the current top frame's register window. Must ALWAYS be recomputed after
/// every `thread.regs` resize/truncate so callers never hold a stale pointer across a realloc.
#[inline(always)]
fn window<'gc>(
    thread: &mut ThreadState<'gc>,
    chunks: &IdVec<BodyId, Chunk>,
) -> (*mut Val<'gc>, usize) {
    let f = thread.frames.last().unwrap();
    let base = f.base;
    let count = chunks[f.chunk].regs as usize;
    debug_assert!(base + count <= thread.regs.len());
    // SAFETY: the top frame's window is regs[base..base + count], same as we measure
    (unsafe { thread.regs.as_mut_ptr().add(base) }, count)
}

/// Run one op against the current frame's register window. This is the main guy!
///
/// What goes in the direct hot match below and what gets placed in cold matters a _lot_. The
/// stack frame here is shared by every arm (sized by the fattest one) and set up on every
/// dispatched op, so one fat arm taxes all of them. Use the op-count feature
/// (`--features op-count`) to identify how much an op is being used within a given run. For any
/// change you make, you should check the prologue:
///
/// ```text
/// otool -tv -p (nm target/release/mimas | grep step_one | awk '{print $3}') target/release/mimas | head -4
/// ```
///
/// The expected frame size is currently 96 bytes (0x60). Raising that is bad!
///
/// Additionally, avoid any (non-inlined) calls anywhere but the tail position, as to not force
/// stack homes for values that don't otherwise need them.
///
/// Generally speaking, ops that just carry registers, do some basic operations, and perform
/// reads/writes are safe for hot dispatch. Anything that allocates or needs variable-length
/// scratch goes to `cold_dispatch` via the wildcard arm -- or, for a hot op with a rare slow
/// path, a `#[cold]` tail-call helper like the `bin_cold` family. (The Call arms look like a
/// violation but aren't: their SmallVec is built straight into the `Flow` return slot, which
/// lives in the caller's frame, not this one.)
#[inline(never)]
fn step_one<'gc>(
    regs: &mut [Val<'gc>],
    code: &mut Decoder,
    ctx: Ctx<'gc>,
    strs: &StrInterner,
) -> RtResult<Flow<'gc>> {
    match OpCode::decode(code) {
        OpCode::LoadConst => {
            let reg = Reg::decode(code);
            let con = Constant::decode(code);
            let val = constant_to_val(con, ctx, strs);
            wr!(regs, reg, val);
        }
        OpCode::Move => {
            let dst = Reg::decode(code);
            let src = Reg::decode(code);
            wr!(regs, dst, rd!(regs, src));
        }
        OpCode::Jump => {
            code.ip = code.u32() as usize;
        }
        OpCode::JumpIf => {
            let cond = Reg::decode(code);
            let target = code.u32() as usize;
            let is_true = code.u8();
            if rd!(regs, cond) == Val::Bool(is_true != 0) {
                code.ip = target;
            }
        }
        OpCode::ForNext => {
            let idx = Reg::decode(code);
            let bound = Reg::decode(code);
            let target = code.u32() as usize;
            let Val::Int(i) = rd!(regs, idx) else {
                unreachable!("for_next idx is statically int")
            };
            let Val::Int(b) = rd!(regs, bound) else {
                unreachable!("for_next bound is statically int")
            };
            let i = i + 1;
            wr!(regs, idx, Val::Int(i));
            if i < b {
                code.ip = target;
            }
        }
        OpCode::GetIndex => {
            let dst = Reg::decode(code);
            let set = Reg::decode(code);
            let index = Reg::decode(code);
            let kind = AccessKind::decode(code);
            let v = get_index(ctx, rd!(regs, set), rd!(regs, index), kind)?;
            wr!(regs, dst, v);
        }
        OpCode::SetIndex => {
            let set = Reg::decode(code);
            let index = Reg::decode(code);
            let value = Reg::decode(code);
            set_index(ctx, rd!(regs, set), rd!(regs, index), rd!(regs, value))?;
        }
        OpCode::GetField => {
            let dst = Reg::decode(code);
            let src = Reg::decode(code);
            let slot = code.u32() as usize;
            let kind = AccessKind::decode(code);
            let receiver = rd!(regs, src);
            if kind == AccessKind::Option && receiver == Val::Null {
                wr!(regs, dst, Val::Null);
                return Ok(Flow::Next);
            }
            let v = match receiver {
                Val::Instance(i) => i.0.borrow().fields[slot],
                Val::Array(a) => a.0.borrow()[slot],
                _ => todo!(),
            };
            wr!(regs, dst, v);
        }
        OpCode::SetField => {
            let receiver_reg = Reg::decode(code);
            let slot = code.u32() as usize;
            let value_reg = Reg::decode(code);
            let receiver = rd!(regs, receiver_reg);
            let value = rd!(regs, value_reg);
            match receiver {
                Val::Instance(i) => i.0.borrow_mut(&ctx).fields[slot] = value,
                Val::Array(a) => a.0.borrow_mut(&ctx)[slot] = value,
                _ => todo!(),
            }
        }
        OpCode::Push => {
            let array_reg = Reg::decode(code);
            let value_reg = Reg::decode(code);
            let arr = rd!(regs, array_reg).as_array().unwrap();
            let value = rd!(regs, value_reg);
            arr.0.borrow_mut(&ctx).push(value);
        }
        OpCode::Len => {
            let dst = Reg::decode(code);
            let src = Reg::decode(code);
            let len = match rd!(regs, src) {
                Val::Array(a) => a.0.borrow().len(),
                Val::Dict(d) => d.0.borrow().len(),
                Val::Str(s) => s.as_str().chars().count(),
                Val::Int(i) => i as usize,
                _ => todo!(),
            };
            wr!(regs, dst, Val::Int(len as i64));
        }
        OpCode::ToFloat => {
            let dst = Reg::decode(code);
            let Val::Int(i) = rd!(regs, Reg::decode(code)) else {
                unreachable!("to_float can only be placed on an int by the compiler!")
            };
            wr!(regs, dst, Val::Float(i as f64));
        }
        OpCode::Sqrt => {
            let dst = Reg::decode(code);
            let Val::Float(f) = rd!(regs, Reg::decode(code)) else {
                unreachable!("to_float can only be placed on a float by the compiler!")
            };
            wr!(regs, dst, Val::Float(f.sqrt()));
        }
        OpCode::Unwrap => {
            let dst = Reg::decode(code);
            let src = Reg::decode(code);
            let v = rd!(regs, src);
            match v {
                Val::Null => return Err(RtErr::UnwrappedNull),
                Val::Raised(err) => {
                    return Err(RtErr::UnwrappedRaised(err.as_str().to_string()));
                }
                _ => wr!(regs, dst, v),
            }
        }
        OpCode::In => {
            let dst = Reg::decode(code);
            let needle = Reg::decode(code);
            let haystack = Reg::decode(code);
            let condition = bool::decode(code);
            let v = contains(rd!(regs, needle), rd!(regs, haystack), condition);
            wr!(regs, dst, v);
        }
        OpCode::LoadBody => {
            let dst = Reg::decode(code);
            let body = BodyId::decode(code);
            wr!(regs, dst, Val::Fn(body));
        }
        OpCode::Call => {
            let dst = Reg::decode(code);
            let callee = Reg::decode(code);
            let len = code.u8() as usize;
            let target = match rd!(regs, callee) {
                Val::Fn(body) => CallTarget::Value(body),
                Val::Closure(closure) => CallTarget::Closure(closure),
                other => return Err(not_callable(other)),
            };
            let mut args = SmallVec::<[Val; 8]>::new();
            for _ in 0..len {
                let r = Reg::decode(code);
                args.push(rd!(regs, r));
            }
            return Ok(Flow::Call { target, dst, args });
        }
        OpCode::CallDirect => {
            let dst = Reg::decode(code);
            let body = BodyId::decode(code);
            let len = code.u8() as usize;
            let mut args = SmallVec::<[Val; 8]>::new();
            for _ in 0..len {
                let r = Reg::decode(code);
                args.push(rd!(regs, r));
            }
            return Ok(Flow::Call {
                target: CallTarget::Fn(body),
                dst,
                args,
            });
        }
        OpCode::Return => {
            let reg = Reg::decode(code);
            return Ok(Flow::Return(rd!(regs, reg)));
        }
        OpCode::CallNative => {
            let dst = Reg::decode(code);
            let id = api::NativeId::decode(code);
            let len = code.u8() as usize;
            let mut args = SmallVec::<[Val; 8]>::new();
            for _ in 0..len {
                let reg = Reg::decode(code);
                args.push(rd!(regs, reg));
            }
            let native = {
                let table = ctx.state().natives.borrow();
                *table
                    .get(id.index())
                    .and_then(|o| o.as_ref())
                    .expect("native id has no installed entry")
            };
            let v = native.call(ctx, &args)?;
            wr!(regs, dst, v);
        }
        OpCode::BoolEq => {
            let dst = Reg::decode(code);
            let l = Reg::decode(code);
            let r = Reg::decode(code);
            let Val::Bool(l) = rd!(regs, l) else {
                unreachable!("illegal bool eq op")
            };
            let Val::Bool(r) = rd!(regs, r) else {
                unreachable!("illegal bool eq op");
            };
            wr!(regs, dst, Val::Bool(l == r));
        }
        OpCode::BoolNe => {
            let dst = Reg::decode(code);
            let l = Reg::decode(code);
            let r = Reg::decode(code);
            let Val::Bool(l) = rd!(regs, l) else {
                unreachable!("illegal bool ne op")
            };
            let Val::Bool(r) = rd!(regs, r) else {
                unreachable!("illegal bool ne op");
            };
            wr!(regs, dst, Val::Bool(l != r));
        }
        OpCode::AddInt => int_arith!(regs, code, ctx, checked_add, BinOp::Add),
        OpCode::ModInt => {
            // unique since right now the None -> integer overflow, but this is mod by zero
            // which is different, and annoying, and ugly
            let dst = Reg::decode(code);
            let left = Reg::decode(code);
            let right = Reg::decode(code);
            match (rd!(regs, left), rd!(regs, right)) {
                (Val::Int(a), Val::Int(b)) => {
                    if b == 0 {
                        return Err(RtErr::ModByZero);
                    }
                    wr!(regs, dst, Val::Int(a % b));
                }
                _ => return bin_cold(regs, dst, left, right, ctx, BinOp::Mod),
            }
        }
        OpCode::SubInt => int_arith!(regs, code, ctx, checked_sub, BinOp::Sub),
        OpCode::MultInt => int_arith!(regs, code, ctx, checked_mul, BinOp::Mult),
        OpCode::IntLt => int_eval!(regs, code, ctx, <, BinOp::LessThan),
        OpCode::IntLe => int_eval!(regs, code, ctx, <=, BinOp::LessEqual),
        OpCode::IntGt => int_eval!(regs, code, ctx, >, BinOp::GreaterThan),
        OpCode::IntGe => int_eval!(regs, code, ctx, >=, BinOp::GreaterEqual),
        OpCode::IntEq => int_eval!(regs, code, ctx, ==, BinOp::Identity),
        OpCode::IntNe => int_eval!(regs, code, ctx, !=, BinOp::NotEqual),
        OpCode::AddFloat => float_arith!(regs, code, ctx, +, BinOp::Add),
        OpCode::SubFloat => float_arith!(regs, code, ctx, -, BinOp::Sub),
        OpCode::MultFloat => float_arith!(regs, code, ctx, *, BinOp::Mult),
        OpCode::DivFloat => float_arith!(regs, code, ctx, /, BinOp::Div),
        OpCode::FloatLt => float_eval!(regs, code, ctx, <, BinOp::LessThan),
        OpCode::FloatLe => float_eval!(regs, code, ctx, <=, BinOp::LessEqual),
        OpCode::FloatGt => float_eval!(regs, code, ctx, >, BinOp::GreaterThan),
        OpCode::FloatGe => float_eval!(regs, code, ctx, >=, BinOp::GreaterEqual),
        OpCode::FloatEq => float_eval!(regs, code, ctx, ==, BinOp::Identity),
        OpCode::FloatNe => float_eval!(regs, code, ctx, !=, BinOp::NotEqual),
        OpCode::StrEq => str_eval!(regs, code, ctx, ==, BinOp::Identity),
        OpCode::StrNe => str_eval!(regs, code, ctx, !=, BinOp::NotEqual),
        OpCode::BIntLt => branch_int!(regs, code, ctx, <, BinOp::LessThan),
        OpCode::BIntLe => branch_int!(regs, code, ctx, <=, BinOp::LessEqual),
        OpCode::BIntGt => branch_int!(regs, code, ctx, >, BinOp::GreaterThan),
        OpCode::BIntGe => branch_int!(regs, code, ctx, >=, BinOp::GreaterEqual),
        OpCode::BIntEq => branch_int!(regs, code, ctx, ==, BinOp::Identity),
        OpCode::BIntNe => branch_int!(regs, code, ctx, !=, BinOp::NotEqual),
        OpCode::AddIntImm => int_arith_imm!(regs, code, ctx, checked_add, BinOp::Add),
        OpCode::SubIntImm => int_arith_imm!(regs, code, ctx, checked_sub, BinOp::Sub),
        OpCode::MultIntImm => int_arith_imm!(regs, code, ctx, checked_mul, BinOp::Mult),
        OpCode::ModIntImm => {
            // see above ModInt, still annoying, still ugly
            let dst = Reg::decode(code);
            let left = Reg::decode(code);
            let val = code.i64();
            match rd!(regs, left) {
                Val::Int(a) => {
                    if val == 0 {
                        return Err(RtErr::ModByZero);
                    }
                    wr!(regs, dst, Val::Int(a % val));
                }
                _ => return bin_cold_imm_int(regs, dst, left, val, ctx, BinOp::Mod),
            }
        }
        OpCode::IntLtImm => int_eval_imm!(regs, code, ctx, <, BinOp::LessThan),
        OpCode::IntLeImm => int_eval_imm!(regs, code, ctx, <=, BinOp::LessEqual),
        OpCode::IntGtImm => int_eval_imm!(regs, code, ctx, >, BinOp::GreaterThan),
        OpCode::IntGeImm => int_eval_imm!(regs, code, ctx, >=, BinOp::GreaterEqual),
        OpCode::IntEqImm => int_eval_imm!(regs, code, ctx, ==, BinOp::Identity),
        OpCode::IntNeImm => int_eval_imm!(regs, code, ctx, !=, BinOp::NotEqual),
        OpCode::BIntLtImm => branch_int_imm!(regs, code, ctx, <, BinOp::LessThan),
        OpCode::BIntLeImm => branch_int_imm!(regs, code, ctx, <=, BinOp::LessEqual),
        OpCode::BIntGtImm => branch_int_imm!(regs, code, ctx, >, BinOp::GreaterThan),
        OpCode::BIntGeImm => branch_int_imm!(regs, code, ctx, >=, BinOp::GreaterEqual),
        OpCode::BIntEqImm => branch_int_imm!(regs, code, ctx, ==, BinOp::Identity),
        OpCode::BIntNeImm => branch_int_imm!(regs, code, ctx, !=, BinOp::NotEqual),
        OpCode::BFloatLt => branch_float!(regs, code, ctx, <, BinOp::LessThan),
        OpCode::BFloatLe => branch_float!(regs, code, ctx, <=, BinOp::LessEqual),
        OpCode::BFloatGt => branch_float!(regs, code, ctx, >, BinOp::GreaterThan),
        OpCode::BFloatGe => branch_float!(regs, code, ctx, >=, BinOp::GreaterEqual),
        OpCode::BFloatEq => branch_float!(regs, code, ctx, ==, BinOp::Identity),
        OpCode::BFloatNe => branch_float!(regs, code, ctx, !=, BinOp::NotEqual),
        OpCode::AddFloatImm => float_arith_imm!(regs, code, ctx, +, BinOp::Add),
        OpCode::SubFloatImm => float_arith_imm!(regs, code, ctx, -, BinOp::Sub),
        OpCode::MultFloatImm => float_arith_imm!(regs, code, ctx, *, BinOp::Mult),
        OpCode::ModFloatImm => float_arith_imm!(regs, code, ctx, %, BinOp::Mod),
        OpCode::FloatLtImm => float_eval_imm!(regs, code, ctx, <, BinOp::LessThan),
        OpCode::FloatLeImm => float_eval_imm!(regs, code, ctx, <=, BinOp::LessEqual),
        OpCode::FloatGtImm => float_eval_imm!(regs, code, ctx, >, BinOp::GreaterThan),
        OpCode::FloatGeImm => float_eval_imm!(regs, code, ctx, >=, BinOp::GreaterEqual),
        OpCode::FloatEqImm => float_eval_imm!(regs, code, ctx, ==, BinOp::Identity),
        OpCode::FloatNeImm => float_eval_imm!(regs, code, ctx, !=, BinOp::NotEqual),
        OpCode::BFloatLtImm => branch_float_imm!(regs, code, ctx, <, BinOp::LessThan),
        OpCode::BFloatLeImm => branch_float_imm!(regs, code, ctx, <=, BinOp::LessEqual),
        OpCode::BFloatGtImm => branch_float_imm!(regs, code, ctx, >, BinOp::GreaterThan),
        OpCode::BFloatGeImm => branch_float_imm!(regs, code, ctx, >=, BinOp::GreaterEqual),
        OpCode::BFloatEqImm => branch_float_imm!(regs, code, ctx, ==, BinOp::Identity),
        OpCode::BFloatNeImm => branch_float_imm!(regs, code, ctx, !=, BinOp::NotEqual),
        op => return cold_dispatch(code, regs, ctx, op, strs),
    }
    Ok(Flow::Next)
}

/// Push a new frame for `body`: grow `regs`, copy args into the param registers and captures into
/// the capture registers, save the caller's ip, and jump. The window pointer in `run_dispatch` is
/// stale after the `resize` here, which is why the driver re-derives it on the next `'frame` pass.
fn enter_call<'gc>(
    thread: &mut ThreadState<'gc>,
    code: &mut Decoder,
    chunks: &IdVec<BodyId, Chunk>,
    body: BodyId,
    dst: Reg,
    args: &[Val<'gc>],
    captures: &[Val<'gc>],
) -> Result<(), RtErr> {
    let chunk = &chunks[body];
    // a dynamic call reaches here with whatever the script had in hand, so this is a real check
    // rather than an invariant -- entering with the wrong count would read foreign registers
    if args.len() != chunk.args as usize {
        return Err(RtErr::WrongArity {
            wanted: chunk.args as usize,
            got: args.len(),
        });
    }
    debug_assert_eq!(captures.len(), chunk.captures.len());
    let new_base = thread.regs.len();
    // hiiiiighwayyyyy toooo theeeee danger zone (be very careful now lol)
    thread
        .regs
        .resize(new_base + chunk.regs as usize, Val::Null);
    for (param_reg, &arg) in chunk.params.iter().zip(args) {
        thread.regs[new_base + param_reg.index()] = arg;
    }
    for (cap_reg, &cap) in chunk.captures.iter().zip(captures) {
        thread.regs[new_base + cap_reg.index()] = cap;
    }
    thread.frames.last_mut().unwrap().ip = code.ip;
    thread.frames.push(Frame {
        chunk: body,
        ip: chunk.offset,
        return_reg: dst.index() as u32,
        base: new_base,
    });
    code.ip = chunk.offset;
    Ok(())
}

/// The callee of a dynamic call wasn't a fn or a closure. Cold so the `Call` arm's shared frame
/// doesn't pay for the capture.
#[cold]
#[inline(never)]
fn not_callable(callee: Val<'_>) -> RtErr {
    RtErr::NotCallable {
        callee: callee.capture(),
    }
}

/// The argument values of a host call, with any trailing parameter the caller left off filled from
/// the fn's own default.
fn call_args<'gc>(
    ctx: Ctx<'gc>,
    strs: &StrInterner,
    f: &Function,
    args: impl Args,
) -> Vec<Val<'gc>> {
    let mut values = args.into_values(ctx);
    for default in f.defaults.iter().skip(values.len()).flatten() {
        values.push(constant_to_val(default.clone(), ctx, strs));
    }
    values
}

/// Run `body` to its return as if the entry frame had called it and hand back what it returned.
/// A fault unwinds to the pre-call depth so the Vm is still usable afterwards.
#[allow(clippy::too_many_arguments)]
fn inject_call<'gc>(
    ctx: Ctx<'gc>,
    code: &mut Decoder,
    chunks: &IdVec<BodyId, Chunk>,
    signatures: &IdVec<BodyId, Option<Function>>,
    strs: &StrInterner,
    sources: &Sources,
    body: BodyId,
    values: &[Val<'gc>],
    captures: &[Val<'gc>],
) -> Result<Val<'gc>, Error> {
    let mut thread = ctx.thread().borrow_mut(&ctx);
    let base = thread.frames.last().unwrap().base;
    let (depth, regs, ip) = (thread.frames.len(), thread.regs.len(), code.ip);
    // the callee returns into r0 of the entry body, which still needs its own value
    let r0 = thread.regs[base];
    // nothing is mutated before this fails, so the thread is untouched and needs no unwinding
    enter_call(&mut thread, code, chunks, body, Reg::ZERO, values, captures).map_err(Error::msg)?;
    let ran = run_dispatch(
        ctx,
        code,
        chunks,
        signatures,
        strs,
        sources,
        &mut thread,
        usize::MAX,
        depth + 1,
    );
    // a fault returns without unwinding, so the callee's frames are still stacked
    if ran.is_err() {
        thread.frames.truncate(depth);
        thread.regs.truncate(regs);
        code.ip = ip;
    }
    let value = std::mem::replace(&mut thread.regs[base], r0);
    drop(thread);
    ran?;
    Ok(value)
}

/// Attach a source location to a runtime fault. `op_ip` is the byte the faulting op was decoded
/// from, the current top frame is still the one that faulted.
#[cold]
fn locate(
    kind: RtErr,
    op_ip: usize,
    thread: &ThreadState<'_>,
    chunks: &IdVec<BodyId, Chunk>,
    sources: &Sources,
) -> Error {
    let frame_chunk = thread.frames.last().unwrap().chunk;
    let chunk = &chunks[frame_chunk];
    let rel = u32::try_from(op_ip - chunk.offset).unwrap();
    let loc = chunk.loc_at(rel);
    // synthetic locs (no real source) point at an empty stub source -- nothing meaningful to
    // highlight, but the kind's title still renders.
    let (src, at) = if loc.is_synthetic() {
        (
            miette::NamedSource::new("<synthetic>", Arc::<str>::from("")),
            miette::SourceSpan::from(0..0),
        )
    } else {
        let source = sources
            .get(&loc.file_id)
            .cloned()
            .unwrap_or_else(|| miette::NamedSource::new("<unknown>", Arc::<str>::from("")));
        (source, loc.into())
    };
    LocatedRtErr { src, at, kind }.into()
}

#[inline(always)]
fn get_index<'gc>(
    ctx: Ctx<'gc>,
    set: Val<'gc>,
    index: Val<'gc>,
    kind: AccessKind,
) -> RtResult<Val<'gc>> {
    if kind == AccessKind::Option && set == Val::Null {
        return Ok(Val::Null);
    }
    fn pos(i: i64, len: usize) -> RtResult<usize> {
        let u = usize::try_from(i).map_err(|_| RtErr::IndexOutOfBounds)?;
        if u >= len {
            Err(RtErr::IndexOutOfBounds)?
        }
        Ok(u)
    }
    Ok(match (set, index) {
        (Val::Array(a), Val::Int(i)) => {
            let v = a.0.borrow();
            v[pos(i, v.len())?]
        }
        (Val::Dict(d), Val::Str(key)) => d.0.borrow().get(&key).copied().unwrap_or(Val::Null),
        (Val::Dict(d), Val::Int(i)) => {
            let m = d.0.borrow();
            let (k, v) = m.entry_at(pos(i, m.len())?);
            Val::Array(ctx.new_array(vec![Val::Str(k), v]))
        }
        (Val::Instance(inst), Val::Int(i)) => {
            let inst = inst.0.borrow();
            inst.fields[pos(i, inst.fields.len())?]
        }
        (Val::Str(s), Val::Int(i)) => {
            let st = s.as_str();
            let ch = if st.is_ascii() {
                st.as_bytes()[pos(i, st.len())?] as char
            } else {
                let p = pos(i, st.chars().count())?;
                st.chars().nth(p).ok_or(RtErr::IndexOutOfBounds)?
            };
            Val::Str(ctx.intern(ch.encode_utf8(&mut [0; 4])))
        }
        (Val::Int(_), Val::Int(_)) => index,
        _ => return Err(RtErr::invalid_index(set, index)),
    })
}

#[inline(always)]
fn set_index<'gc>(ctx: Ctx<'gc>, set: Val<'gc>, index: Val<'gc>, value: Val<'gc>) -> RtResult<()> {
    #[inline(always)]
    fn pos(i: i64, len: usize) -> RtResult<usize> {
        let u = usize::try_from(i).map_err(|_| RtErr::IndexOutOfBounds)?;
        if u >= len {
            Err(RtErr::IndexOutOfBounds)?
        }
        Ok(u)
    }
    match (set, index) {
        (Val::Array(a), Val::Int(i)) => {
            let mut v = a.0.borrow_mut(&ctx);
            let p = pos(i, v.len())?;
            v[p] = value;
        }
        (Val::Dict(d), Val::Str(key)) => {
            d.0.borrow_mut(&ctx).insert(key, value);
        }
        (Val::Instance(inst), Val::Int(i)) => {
            let mut inst = inst.0.borrow_mut(&ctx);
            let p = pos(i, inst.fields.len())?;
            inst.fields[p] = value;
        }
        _ => return Err(RtErr::invalid_index(set, index)),
    }
    Ok(())
}

fn contains<'gc>(needle: Val<'gc>, haystack: Val<'gc>, condition: bool) -> Val<'gc> {
    let c = match haystack {
        Val::Array(a) => a.0.borrow().contains(&needle),
        Val::Dict(d) => {
            let Val::Str(key) = needle else { todo!() };
            d.0.borrow().contains_key(&key)
        }
        Val::Str(s) => {
            let Val::Str(n) = needle else { todo!() };
            s.as_str().contains(n.as_str())
        }
        _ => todo!(),
    };
    Val::Bool(c == condition)
}

// Each of these absorbs everything the arm would otherwise do inline on the slow path: the generic
// `bin`/`unary` call, the register write (or branch / ip update), and -- because they return
// `RtResult<Flow>` directly -- the error widening. The fast-path macros reach them with `return`,
// so the arm forwards this result verbatim. That keeps the operands `Val`-by-value (never spilled
// across the tag check in the hot arm) and collapses the per-arm error epilogues into one shared
// tail. They MUST stay trivial past the `bin`/`unary` call: anything that takes the address of a
// local in here reintroduces an escape (in this frame, harmless to `step_one`, but don't let these
// balloon and then get inlined).

#[cold]
#[inline(never)]
fn bin_cold<'gc>(
    regs: &mut [Val<'gc>],
    dst: Reg,
    left: Reg,
    right: Reg,
    ctx: Ctx<'gc>,
    op: BinOp,
) -> RtResult<Flow<'gc>> {
    let v = crate::val::bin(rd!(regs, left), ctx, rd!(regs, right), op)?;
    wr!(regs, dst, v);
    Ok(Flow::Next)
}

#[cold]
#[inline(never)]
fn bin_cold_imm_int<'gc>(
    regs: &mut [Val<'gc>],
    dst: Reg,
    left: Reg,
    val: i64,
    ctx: Ctx<'gc>,
    op: BinOp,
) -> RtResult<Flow<'gc>> {
    let v = crate::val::bin(rd!(regs, left), ctx, Val::Int(val), op)?;
    wr!(regs, dst, v);
    Ok(Flow::Next)
}

#[cold]
#[inline(never)]
fn bin_cold_imm_float<'gc>(
    regs: &mut [Val<'gc>],
    dst: Reg,
    left: Reg,
    val: f64,
    ctx: Ctx<'gc>,
    op: BinOp,
) -> RtResult<Flow<'gc>> {
    let v = crate::val::bin(rd!(regs, left), ctx, Val::Float(val), op)?;
    wr!(regs, dst, v);
    Ok(Flow::Next)
}

#[cold]
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn branch_cold<'gc>(
    regs: &mut [Val<'gc>],
    code: &mut Decoder,
    target: usize,
    is_true: bool,
    left: Reg,
    right: Reg,
    ctx: Ctx<'gc>,
    op: BinOp,
) -> RtResult<Flow<'gc>> {
    let hit = matches!(
        crate::val::bin(rd!(regs, left), ctx, rd!(regs, right), op)?,
        Val::Bool(true)
    );
    if hit == is_true {
        code.ip = target;
    }
    Ok(Flow::Next)
}

#[cold]
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn branch_cold_imm_int<'gc>(
    regs: &mut [Val<'gc>],
    code: &mut Decoder,
    target: usize,
    is_true: bool,
    left: Reg,
    val: i64,
    ctx: Ctx<'gc>,
    op: BinOp,
) -> RtResult<Flow<'gc>> {
    let hit = matches!(
        crate::val::bin(rd!(regs, left), ctx, Val::Int(val), op)?,
        Val::Bool(true)
    );
    if hit == is_true {
        code.ip = target;
    }
    Ok(Flow::Next)
}

#[cold]
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn branch_cold_imm_float<'gc>(
    regs: &mut [Val<'gc>],
    code: &mut Decoder,
    target: usize,
    is_true: bool,
    left: Reg,
    val: f64,
    ctx: Ctx<'gc>,
    op: BinOp,
) -> RtResult<Flow<'gc>> {
    let hit = matches!(
        crate::val::bin(rd!(regs, left), ctx, Val::Float(val), op)?,
        Val::Bool(true)
    );
    if hit == is_true {
        code.ip = target;
    }
    Ok(Flow::Next)
}

/// The genuinely-cold ops: ones that allocate, build collections, call out, or are otherwise rare
/// enough that keeping their bodies (and their large per-op scratch) out of `step_one`'s frame is
/// strictly a win. Reached via the wildcard arm with `return cold_dispatch(...)`.
#[cold]
#[inline(never)]
fn cold_dispatch<'gc>(
    code: &mut Decoder,
    regs: &mut [Val<'gc>],
    ctx: Ctx<'gc>,
    op: OpCode,
    strs: &StrInterner,
) -> RtResult<Flow<'gc>> {
    match op {
        OpCode::Bin => {
            let dst = Reg::decode(code);
            let left = Reg::decode(code);
            let op = BinOp::decode(code);
            let right = Reg::decode(code);
            let v = crate::val::bin(rd!(regs, left), ctx, rd!(regs, right), op)?;
            wr!(regs, dst, v);
        }
        OpCode::Unary => {
            let dst = Reg::decode(code);
            let op = UnaryOp::decode(code);
            let src = Reg::decode(code);
            let v = crate::val::unary(rd!(regs, src), ctx, op)?;
            wr!(regs, dst, v);
        }
        OpCode::Switch => {
            let scrut = Reg::decode(code);
            let base = code.u32();
            let default = code.u32() as usize;
            let len = code.u16() as usize;
            let table = code.ip;
            let target = match rd!(regs, scrut) {
                Val::Instance(i) => {
                    let idx = i.0.borrow().struct_id.wrapping_sub(base) as usize;
                    if idx < len {
                        code.peek_u32(table + idx * 4) as usize
                    } else {
                        default
                    }
                }
                Val::Int(v) => {
                    let idx = v.wrapping_sub(base as i64);
                    if idx >= 0 && (idx as usize) < len {
                        code.peek_u32(table + idx as usize * 4) as usize
                    } else {
                        default
                    }
                }
                _ => default,
            };
            code.ip = target;
        }
        OpCode::NewArray => {
            let dst = Reg::decode(code);
            wr!(regs, dst, Val::Array(ctx.new_array(Vec::new())));
        }
        OpCode::NewDict => {
            let dst = Reg::decode(code);
            wr!(regs, dst, Val::Dict(ctx.new_dict(DictMap::new())));
        }
        OpCode::Insert => {
            let dict_reg = Reg::decode(code);
            let key_id = shared::StrId::from(code.u32());
            let value_reg = Reg::decode(code);
            let dict = rd!(regs, dict_reg).as_dict().unwrap();
            let value = rd!(regs, value_reg);
            let key = ctx.intern(strs.get(key_id));
            dict.0.borrow_mut(&ctx).insert(key, value);
        }
        OpCode::Format => {
            let dst = Reg::decode(code);
            let len = code.u16();
            let mut text = String::with_capacity(32); // gives us a little size just to start
            for _ in 0..len {
                match OpFormatPart::decode(code) {
                    OpFormatPart::Literal(str_id) => text.push_str(strs.get(str_id)),
                    OpFormatPart::Value(reg) => ctx.to_string_into(&mut text, rd!(regs, reg)),
                }
            }
            wr!(regs, dst, Val::Str(ctx.intern(&text)));
        }
        OpCode::NewInstance => {
            let dst = Reg::decode(code);
            let adt = code.u32();
            let len = code.u8() as usize;
            let fields = if len <= INLINE_FIELDS {
                let mut data = [Val::Null; INLINE_FIELDS];
                for slot in data.iter_mut().take(len) {
                    let reg = Reg::decode(code);
                    *slot = rd!(regs, reg);
                }
                Fields::Inline {
                    len: len as u8,
                    data,
                }
            } else {
                let mut v = Vec::with_capacity(len);
                for _ in 0..len {
                    let reg = Reg::decode(code);
                    v.push(rd!(regs, reg));
                }
                Fields::Spilled(v)
            };
            let inst = ctx.new_instance(adt, fields);
            wr!(regs, dst, Val::Instance(inst));
        }
        OpCode::Panic => return Err(RtErr::MatchPanicReached),
        OpCode::IsInstance => {
            let dst = Reg::decode(code);
            let src = Reg::decode(code);
            let adt = code.u32();
            let matches = matches!(
                rd!(regs, src),
                Val::Instance(i) if i.0.borrow().struct_id == adt,
            );
            wr!(regs, dst, Val::Bool(matches));
        }
        OpCode::NewClosure => {
            let dst = Reg::decode(code);
            let body = BodyId::decode(code);
            let len = code.u8() as usize;
            let mut captures = Vec::with_capacity(len);
            for _ in 0..len {
                let reg = Reg::decode(code);
                captures.push(rd!(regs, reg));
            }
            let closure = ctx.new_closure(body, captures);
            wr!(regs, dst, Val::Closure(closure));
        }
        OpCode::Raise => {
            let src = Reg::decode(code);
            let Val::Str(err) = rd!(regs, src) else {
                unreachable!("raise on a non-str value")
            };
            return Ok(Flow::Return(Val::Raised(err)));
        }
        OpCode::IsRaised => {
            let dst = Reg::decode(code);
            let src = Reg::decode(code);
            let v = rd!(regs, src);
            wr!(regs, dst, Val::Bool(matches!(v, Val::Raised(_))));
        }
        OpCode::UnwrapRaised => {
            let dst = Reg::decode(code);
            let src = Reg::decode(code);
            let Val::Raised(err) = rd!(regs, src) else {
                unreachable!("UnwrapRaised on non-raised value")
            };
            wr!(regs, dst, Val::Str(err));
        }
        _ => unreachable!("failed to find cold op"),
    }

    Ok(Flow::Next)
}

/// Convert a compile-time `Constant` into a runtime `Val<'gc>`. Str constants resolve
/// through the chunk's string pool into the arena's interner.
fn constant_to_val<'gc>(c: Constant, ctx: Ctx<'gc>, c_cstrs: &StrInterner) -> Val<'gc> {
    match c {
        Constant::Bool(b) => Val::Bool(b),
        Constant::Int(i) => Val::Int(i),
        Constant::Float(f) => Val::Float(f),
        Constant::Str(id) => Val::Str(ctx.intern(c_cstrs.get(id))),
        Constant::Array(items) => {
            let out: Vec<Val<'gc>> = items
                .into_iter()
                .map(|c| constant_to_val(c, ctx, c_cstrs))
                .collect();
            Val::Array(ctx.new_array(out))
        }
        Constant::Null => Val::Null,
    }
}

impl Vm {
    pub fn install_library<F>(&mut self, install_fn: F) -> ::api::Library<()>
    where
        F: for<'gc> FnOnce(&mut crate::api::Api<'_, 'gc>),
    {
        self.arena.mutate(|mc, state| {
            let ctx = state.ctx(mc);
            crate::api::install_into(ctx, install_fn)
        })
    }

    /// Look up a top-level local by name from the entry frame and snapshot it into a
    /// gc-free [`Captured`] tree. The arena's `'gc` keeps `Val<'gc>` from escaping; the
    /// snapshot is taken inside `arena.mutate` so the full structure (not just scalars)
    /// can safely cross the boundary.
    pub fn resolve_name(&mut self, lexeme: &str) -> Option<Captured> {
        let chunks = &self.chunks;
        self.arena.mutate(|_mc, state| {
            let t = state.thread.borrow();
            let frame = t.frames.first()?;
            let reg = *chunks[frame.chunk].locals.get(lexeme)?;
            t.regs
                .get(frame.base + reg.index())
                .copied()
                .map(Val::capture)
        })
    }

    /// The script's items as the host sees them: its fns, consts and types, and the same for
    /// every module. Privacy is a rule for scripts, not for the host, so private items are here
    /// too -- a `Type` carries its own `vis` if you care to check.
    pub fn root(&self) -> &Module {
        &self.root
    }

    /// The host types this program was compiled against, for turning a Rust type into its `Ty`.
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// The signature of a script fn or closure `body`, for a function value the host only knows
    /// by its [`BodyId`]. `None` for a body with no fn type of its own.
    pub fn signature(&self, body: BodyId) -> Option<&FnHeader> {
        self.signatures.get(body)?.as_ref().map(|f| &f.header)
    }

    /// Calls a fn by `::` path (`"update"`, `"game::tick"`) once the program has [run](Self::run).
    /// Arguments and `R` are checked against the signature first; trailing defaults may be left
    /// out; a fault leaves the Vm usable.
    pub fn call<R: for<'gc> MimasType<'gc>>(
        &mut self,
        path: &str,
        args: impl Args,
    ) -> Result<R, Error> {
        self.call_then(path, args, |_| ()).map(|(value, ())| value)
    }

    /// [`call`](Self::call), then `then` with the arena still open -- the fn's frame is gone but
    /// nothing has been collected yet, so whatever a native put aside during the call can still
    /// be read. `then` doesn't run when the call faults.
    pub fn call_then<R, T>(
        &mut self,
        path: &str,
        args: impl Args,
        then: impl for<'gc> FnOnce(Ctx<'gc>) -> T,
    ) -> Result<(R, T), Error>
    where
        R: for<'gc> MimasType<'gc>,
    {
        let root = Rc::clone(&self.root);
        let Some(f) = root.function(path) else {
            return Err(miette::miette!("no fn `{path}` is reachable from the host"));
        };
        f.check(
            path,
            &args.tys(&self.registry),
            R::mimas_ty(&self.registry).as_ref(),
        )?;
        self.call_function(f, args, then)
    }

    /// [`call_then`](Self::call_then) for a fn already looked up in [`root`](Self::root), without
    /// checking `args` or `R` against its signature: [`Function::check`] once, then call as often
    /// as needed. A fn from another Vm is a logic error.
    pub fn call_function<R, T>(
        &mut self,
        f: &Function,
        args: impl Args,
        then: impl for<'gc> FnOnce(Ctx<'gc>) -> T,
    ) -> Result<(R, T), Error>
    where
        R: for<'gc> MimasType<'gc>,
    {
        let Vm {
            code,
            chunks,
            signatures,
            c_strs: strs,
            arena,
            sources,
            ..
        } = self;
        let result = arena.mutate(|mc, state| {
            let ctx = state.ctx(mc);
            let values = call_args(ctx, strs, f, args);
            let value = inject_call(
                ctx,
                code,
                chunks,
                signatures,
                strs,
                sources,
                f.body,
                &values,
                &[],
            )?;
            let value = R::from_value(ctx, value).map_err(|err| Error::msg(RtErr::from(err)))?;
            Ok((value, then(ctx)))
        });
        self.arena.collect_debt();
        result
    }

    /// [`call_function`](Self::call_function) for a function value the script handed over through
    /// [`Ctx::stash`] -- a fn, or a closure, which is called with whatever it captured. Argument
    /// and return types aren't checked: read the shape with [`signature`](Self::signature) first
    /// if the value came from somewhere the host doesn't control.
    pub fn call_value<R: for<'gc> MimasType<'gc>>(
        &mut self,
        f: &Stashed,
        args: impl Args,
    ) -> Result<R, Error> {
        self.call_value_then(f, args, |_| ())
            .map(|(value, ())| value)
    }

    /// [`call_value`](Self::call_value), then `then` with the arena still open, the way
    /// [`call_then`](Self::call_then) follows [`call`](Self::call).
    pub fn call_value_then<R, T>(
        &mut self,
        f: &Stashed,
        args: impl Args,
        then: impl for<'gc> FnOnce(Ctx<'gc>) -> T,
    ) -> Result<(R, T), Error>
    where
        R: for<'gc> MimasType<'gc>,
    {
        let Vm {
            code,
            chunks,
            c_strs: strs,
            arena,
            sources,
            signatures,
            ..
        } = self;
        let result = arena.mutate(|mc, state| {
            let ctx = state.ctx(mc);
            // a handle from a superseded program names a body that means something else now, so
            // it's rejected here rather than left to `fetch`, which panics on one
            if !ctx.holds(f) {
                return Err(miette::miette!(
                    "the host called a function value stashed against a program that is no longer \
                     loaded"
                ));
            }
            let (body, captures) = match ctx.fetch(f) {
                Val::Fn(body) => (body, &[] as &[_]),
                Val::Closure(closure) => (
                    closure.0.function,
                    Gc::as_ref(closure.0).captures.as_slice(),
                ),
                other => {
                    return Err(miette::miette!(
                        "the host called `{}`, which isn't a function",
                        ctx.display(other)
                    ));
                }
            };
            // a tuple struct's name is a `Val::Fn` too, carrying a header the solver synthesized
            // and no body to enter -- having a signature is what tells a real fn apart
            let Some(signature) = signatures.get(body).and_then(Option::as_ref) else {
                return Err(miette::miette!(
                    "the host called `{}`, which has no function body",
                    ctx.display(ctx.fetch(f))
                ));
            };
            // `enter_call` is what rejects a wrong argument count, for the host and script alike
            let values = call_args(ctx, strs, signature, args);
            let value = inject_call(
                ctx, code, chunks, signatures, strs, sources, body, &values, captures,
            )?;
            let value = R::from_value(ctx, value).map_err(|err| Error::msg(RtErr::from(err)))?;
            Ok((value, then(ctx)))
        });
        self.arena.collect_debt();
        result
    }

    pub fn execute<F>(source: &str, install_lib: F) -> std::result::Result<Self, ExecuteError>
    where
        F: for<'gc> FnOnce(&mut crate::api::Api<'_, 'gc>),
    {
        Self::execute_files(&[("<execute>", source)], install_lib)
    }

    /// Multi-file variant of [`Self::execute`]. Each `(name, source)` pair becomes its
    /// own compilation unit; `name` is the module stem (e.g. `("foo", "module @; ...")`
    /// is referenced from another file as `foo::...`). One of the files should be named
    /// `main`, which holds the entry statements.
    pub fn execute_files<F>(
        files: &[(&str, &str)],
        install_lib: F,
    ) -> std::result::Result<Self, ExecuteError>
    where
        F: for<'gc> FnOnce(&mut crate::api::Api<'_, 'gc>),
    {
        let mut vm = Self::compile_files(files, install_lib)?;
        vm.run()?;
        Ok(vm)
    }

    pub fn compile<F>(source: &str, install_lib: F) -> std::result::Result<Self, ExecuteError>
    where
        F: for<'gc> FnOnce(&mut crate::api::Api<'_, 'gc>),
    {
        Self::compile_files(&[("<compile>", source)], install_lib)
    }

    /// Multi-file variant of [`Self::compile`].
    pub fn compile_files<F>(
        files: &[(&str, &str)],
        install_lib: F,
    ) -> std::result::Result<Self, ExecuteError>
    where
        F: for<'gc> FnOnce(&mut crate::api::Api<'_, 'gc>),
    {
        let mut vm = Self::new();
        let library = vm.install_library(install_lib);
        // the first library a host installs is its manifest, when it runs from its cargo target dir
        #[cfg(feature = "export-api")]
        {
            use std::sync::atomic::{AtomicBool, Ordering};
            static WRITTEN: AtomicBool = AtomicBool::new(false);
            if !WRITTEN.swap(true, Ordering::Relaxed)
                && let Some(path) = std::env::current_exe()
                    .ok()
                    .and_then(|exe| ::api::Manifest::host_path(&exe))
                && let Err(e) = write_api(&library, &path)
            {
                eprintln!(
                    "mimas: couldn't write the API manifest to {}: {e}",
                    path.display()
                );
            }
        }
        let (program, sources) = Self::build_program(files, &library)?;

        vm.load_program(program);
        vm.set_sources(sources);
        vm.registry = library.into_registry();
        Ok(vm)
    }

    /// The entire compilation process -- parse, solve and compile `files` against `library`, which
    /// is the set of natives the resulting program is built to line up with.
    fn build_program(
        files: &[(&str, &str)],
        library: &::api::Library<()>,
    ) -> std::result::Result<(Program, Sources), ExecuteError> {
        use solve::Resolutions;

        let mut loaded = solve::Modules::from_files(files.iter().copied(), library);
        // todo: only the first error makes it out
        if !loaded.errors.is_empty() {
            return Err(loaded.errors.swap_remove(0).into());
        }

        let stmts: Vec<_> = loaded
            .asts
            .into_iter()
            .flat_map(|ast| ast.unpack())
            .collect();
        let sources = loaded.sources;
        let resolutions = Resolutions::from(loaded.solver);
        // todo: there's zero reason to clone this here, im just trying to get a working version --
        // there's probably a much smoother way to get the intrinsics over here
        let mut ir = compile::Ir::new(
            resolutions,
            library.intrinsics().iter().map(|(a, b)| (*a, *b)).collect(),
        );
        ir.lower(&stmts);
        Ok((compile::Compiler::new().compile(ir), sources))
    }
}

/// Writes `library` to `path` as a manifest unless its output would be identical to what is
/// already present.
#[cfg(feature = "export-api")]
pub fn write_api(library: &::api::Library<()>, path: &std::path::Path) -> std::io::Result<()> {
    ::api::Manifest::new(library.clone()).write(path)
}

/// Single error surface for [`Vm::execute`]. Every stage (parse, solve, runtime) now emits
/// `miette::Report`s, so they all collapse into one variant; the distinction lives in the
/// `miette::Diagnostic` impl of whatever kind was originally constructed.
#[derive(Debug)]
pub struct ExecuteError(pub Error);

impl From<Error> for ExecuteError {
    fn from(e: Error) -> Self {
        Self(e)
    }
}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

pub use crate::val::Captured;
