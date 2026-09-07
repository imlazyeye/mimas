use api::NativeId;
use itertools::Itertools;
use parse::{
    AccessKind,
    components::{Pat, PatKind},
};
use shared::StrId;
use solve::components::{AdtId, Ty};

use crate::ir::{BodyId, Local, UnaryOp};

use super::{BinOp, BlockId, Constant, InstId, Ir, Lower};

#[derive(Debug, PartialEq, Clone)]
pub enum Inst {
    Constant(Constant),
    SetLocal(Local, InstId),
    GetLocal(Local),
    BinOp {
        left: InstId,
        op: BinOp,
        right: InstId,
        kind: OperandKind,
    },
    UnaryOp {
        op: UnaryOp,
        right: InstId,
    },
    JumpIfFalse {
        condition: InstId,
        target: BlockId,
    },
    Jump {
        target: BlockId,
    },
    Switch {
        scrut: InstId,
        base: u32,
        table: Vec<BlockId>,
        default: BlockId,
    },
    Phi(Vec<(BlockId, InstId)>),
    ForNext {
        idx: InstId,
        bound: InstId,
        target: BlockId,
    },
    NewArray,
    NewDict,
    SetIndex {
        set: InstId,
        index: InstId,
        value: InstId,
    },
    GetIndex {
        set: InstId,
        index: InstId,
        kind: AccessKind,
    },
    GetField {
        src: InstId,
        slot: u32,
        kind: AccessKind,
    },
    SetField {
        receiver: InstId,
        slot: u32,
        value: InstId,
    },
    Push {
        array: InstId,
        value: InstId,
    },
    Insert {
        dict: InstId,
        key: StrId,
        value: InstId,
    },
    Len(InstId),
    ToFloat(InstId),
    Sqrt(InstId),
    Unwrap(InstId),
    In(InstId, InstId, bool),
    Format(Vec<FormatPart>),
    RefBody(BodyId),
    MakeClosure {
        body: BodyId,
        captures: Vec<InstId>,
    },
    Call {
        callee: InstId,
        args: Vec<InstId>,
    },
    CallDirect {
        body: BodyId,
        args: Vec<InstId>,
    },
    CallNative {
        id: NativeId,
        args: Vec<InstId>,
    },
    Return(InstId),
    NewInstance {
        adt: AdtId,
        fields: Vec<InstId>,
    },
    IsInstance {
        src: InstId,
        adt: AdtId,
    },
    Panic,
    Raise(InstId),
    IsRaised(InstId),
    UnwrapRaised(InstId),
}

impl Inst {
    /// Successor of this instruction, if it branches (conditional or not).
    pub(crate) fn jump_target(&self) -> Option<BlockId> {
        match *self {
            Inst::Jump { target }
            | Inst::JumpIfFalse { target, .. }
            | Inst::ForNext { target, .. } => Some(target),
            _ => None,
        }
    }

    /// Mutable handle to the branch target, for redirecting an edge.
    pub(crate) fn jump_target_mut(&mut self) -> Option<&mut BlockId> {
        match self {
            Inst::Jump { target }
            | Inst::JumpIfFalse { target, .. }
            | Inst::ForNext { target, .. } => Some(target),
            _ => None,
        }
    }

    /// Roots are instructions that must run regardless of if their result is used. This could be
    /// configurable in the future.
    pub(crate) fn is_root(&self) -> bool {
        self.has_effect() || self.can_fault()
    }

    /// Whether or not this Inst has a lasting effect (used to determine whether or not its safe for
    /// us to compile out if it's ultimately unused).
    pub(crate) fn has_effect(&self) -> bool {
        match self {
            // mutations, calls, and control flow: must run regardless of whether a result is used.
            Inst::SetLocal(..)
            | Inst::SetIndex { .. }
            | Inst::SetField { .. }
            | Inst::Push { .. }
            | Inst::Insert { .. }
            | Inst::Call { .. }
            | Inst::CallDirect { .. }
            | Inst::CallNative { .. }
            | Inst::Return(..)
            | Inst::Raise(..)
            | Inst::Panic
            | Inst::Jump { .. }
            | Inst::JumpIfFalse { .. }
            | Inst::ForNext { .. }
            | Inst::Switch { .. } => true,
            // pure value producers: droppable when their result is unused.
            Inst::Constant(..)
            | Inst::GetLocal(..)
            | Inst::BinOp { .. }
            | Inst::UnaryOp { .. }
            | Inst::Phi(..)
            | Inst::NewArray
            | Inst::NewDict
            | Inst::GetIndex { .. }
            | Inst::GetField { .. }
            | Inst::Len(..)
            | Inst::ToFloat(..)
            | Inst::Sqrt(..)
            | Inst::Unwrap(..)
            | Inst::In(..)
            | Inst::Format(..)
            | Inst::RefBody(..)
            | Inst::MakeClosure { .. }
            | Inst::NewInstance { .. }
            | Inst::IsInstance { .. }
            | Inst::IsRaised(..)
            | Inst::UnwrapRaised(..) => false,
        }
    }

    /// Whether or not this Inst can trigger a fault. Similar to [Inst::has_effect], we use this to
    /// decide if its safe to compile out. By default we will not, but in a future aggressive
    /// performance mode, we could. Ideally we warn users about dead code though, so this should be
    /// somewhat moot.
    pub(crate) fn can_fault(&self) -> bool {
        match self {
            // raises an RtErr at runtime: arithmetic overflow / div-or-mod by zero / bad shift,
            // negate overflow, out-of-bounds index, unwrapping null or a raised error, a callee
            // faulting, or an explicit match-panic.
            Inst::BinOp { .. }
            | Inst::UnaryOp { .. }
            | Inst::SetIndex { .. }
            | Inst::GetIndex { .. }
            | Inst::Unwrap(..)
            | Inst::UnwrapRaised(..)
            | Inst::Call { .. }
            | Inst::CallDirect { .. }
            | Inst::CallNative { .. }
            | Inst::Panic => true,
            // never faults. field access is slot-checked by the solver; `raise` produces a
            // recoverable Raised value, not a fault.
            Inst::Constant(..)
            | Inst::SetLocal(..)
            | Inst::GetLocal(..)
            | Inst::Phi(..)
            | Inst::NewArray
            | Inst::NewDict
            | Inst::GetField { .. }
            | Inst::SetField { .. }
            | Inst::Push { .. }
            | Inst::Insert { .. }
            | Inst::Len(..)
            | Inst::ToFloat(..)
            | Inst::Sqrt(..)
            | Inst::In(..)
            | Inst::Format(..)
            | Inst::RefBody(..)
            | Inst::MakeClosure { .. }
            | Inst::Return(..)
            | Inst::NewInstance { .. }
            | Inst::IsInstance { .. }
            | Inst::IsRaised(..)
            | Inst::Raise(..)
            | Inst::Jump { .. }
            | Inst::JumpIfFalse { .. }
            | Inst::ForNext { .. }
            | Inst::Switch { .. } => false,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FormatPart {
    Literal(StrId),
    Value(InstId),
}

#[mutants::skip]
impl IrDisplay for Inst {
    fn ir_display(&self, ir: &Ir) -> String {
        match self {
            Inst::Constant(constant) => format!("const {}", constant.ir_display(ir)),
            Inst::SetLocal(variable, inst) => {
                format!("set {variable}: {inst}")
            }
            Inst::GetLocal(variable) => format!("get {variable}"),
            Inst::BinOp {
                left,
                op,
                right,
                kind,
            } => {
                format!("{op} {left}, {right} ({kind:?})")
            }
            Inst::UnaryOp { op, right } => {
                format!("{op} {right}")
            }
            Inst::JumpIfFalse { condition, target } => {
                format!("jump_if_false {condition}, {}", target.ir_display(ir))
            }
            Inst::Jump { target } => format!("jump {}", target.ir_display(ir)),
            Inst::Phi(branches) => {
                let branches = branches
                    .iter()
                    .map(|(block, inst)| format!("{}: {inst}", block.ir_display(ir)))
                    .join(", ");
                format!("phi {branches}")
            }
            Inst::ForNext { idx, bound, target } => {
                format!("for_next {idx} in {bound} -> {}", target.ir_display(ir))
            }
            Inst::NewArray => "new_array".into(),
            Inst::NewDict => "new_dict".into(),
            Inst::SetIndex { set, index, value } => {
                format!("set_index {set} @ {index}: {value}")
            }
            Inst::Push { array, value } => format!("push {array}, {value}"),
            Inst::Insert { dict, key, value } => {
                format!("insert {dict}, {value} @ {}", ir.str(*key))
            }
            Inst::GetIndex { set, index, kind } => {
                format!(
                    "get_index {set} @ {index}{}",
                    if kind == &AccessKind::Direct { "" } else { "?" }
                )
            }
            Inst::GetField { src, slot, kind } => {
                format!(
                    "get_field {src} .{slot}{}",
                    if kind == &AccessKind::Direct { "" } else { "?" }
                )
            }
            Inst::SetField {
                receiver,
                slot,
                value,
            } => {
                format!("set_field {receiver} .{slot}: {value}")
            }
            Inst::Len(inst_id) => format!("len {inst_id}"),
            Inst::ToFloat(inst_id) => format!("to_float {inst_id}"),
            Inst::Sqrt(inst_id) => format!("sqrt {inst_id}"),
            Inst::Unwrap(inst_id) => format!("unwrap {inst_id}"),
            Inst::Raise(inst_id) => format!("raise {inst_id}"),
            Inst::IsRaised(inst_id) => format!("is_raised {inst_id}"),
            Inst::UnwrapRaised(inst_id) => format!("unwrap_raised {inst_id}"),
            Inst::In(lhs, rhs, condition) => {
                format!("{}in {lhs}: {rhs}", if *condition { "" } else { "!" })
            }
            Inst::Format(parts) => {
                let parts = parts
                    .iter()
                    .copied()
                    .map(|part| match part {
                        FormatPart::Literal(str_id) => format!("{:?}", ir.str(str_id)),
                        FormatPart::Value(inst_id) => inst_id.to_string(),
                    })
                    .join(", ");
                format!("format {parts}")
            }
            Inst::RefBody(body) => format!("ref_body @{}", body.index()),
            Inst::MakeClosure { body, captures } => {
                let caps = captures.iter().map(|c| c.to_string()).join(", ");
                format!("make_closure @{} [{caps}]", body.index())
            }
            Inst::Call { callee, args } => {
                let args = args.iter().map(|a| a.to_string()).join(", ");
                format!("call {callee}({args})")
            }
            Inst::CallDirect { body, args } => {
                let args = args.iter().map(|a| a.to_string()).join(", ");
                format!("call_direct @{}({args})", body.index())
            }
            Inst::CallNative { id, args } => {
                let args = args.iter().map(|a| a.to_string()).join(", ");
                format!("call_native #{}({args})", id.index())
            }
            Inst::Return(value) => format!("return {value}"),
            Inst::NewInstance { adt, fields } => {
                format!(
                    "make_instance @{} {{ {} }}",
                    adt.index(),
                    fields.iter().map(|i| i.to_string()).join(", ")
                )
            }
            Inst::IsInstance { src, adt } => format!("is_instance {src} @{}", adt.index()),
            Inst::Switch {
                scrut,
                base,
                table,
                default,
            } => {
                let arms = table
                    .iter()
                    .enumerate()
                    .map(|(i, b)| format!("{}: {}", *base + i as u32, b.ir_display(ir)))
                    .join(", ");
                format!("switch {scrut} [{arms}] else {}", default.ir_display(ir))
            }
            Inst::Panic => "panic".into(),
        }
    }
}

pub(crate) trait IrDisplay {
    fn ir_display(&self, ir: &Ir) -> String;
}

pub(crate) struct BlockWriter<'ir> {
    pub(super) ir: &'ir mut Ir,
}

impl BlockWriter<'_> {
    pub(crate) fn instruct(&mut self, kind: impl Into<Inst>) -> InstId {
        self.ir.push_inst(kind)
    }

    pub(crate) fn emit_expr(&mut self, expr: &parse::Expr) -> Option<InstId> {
        expr.lower(self.ir)
    }

    /// Tests `pat` against `scrut_val`, jumping to `fail_block` on any mismatch (tag, literal,
    /// guard, etc). On success, falls through with any bindings emitted. Caller continues with
    /// the arm body directly -- no separate bool to act on.
    pub fn test_pattern(
        &mut self,
        pat: &Pat,
        scrut_val: InstId,
        fail_block: BlockId,
    ) -> Option<()> {
        match pat.kind() {
            PatKind::Ident(_) => {
                let dec = self.ir.node_dec(pat.id());
                let local = self.ir.local_for(dec);
                self.set_local(local, scrut_val);
                Some(())
            }
            PatKind::Literal(lit) => {
                let con = Constant::from_literal(self.ir, lit.clone());
                let op_kind = match con {
                    Constant::Bool(_) => OperandKind::Bool,
                    Constant::Int(_) => OperandKind::Int,
                    Constant::Float(_) => OperandKind::Float,
                    Constant::Str(_) => OperandKind::Str,
                    Constant::Array(_) | Constant::Null => OperandKind::Generic,
                };
                let this = self.constant(con);
                let eq = self.bin(BinOp::Identity, scrut_val, this, op_kind);
                self.jump_if_false(eq, fail_block);
                Some(())
            }
            PatKind::Tuple(sub_pats) => {
                for (i, sub) in sub_pats.iter().enumerate() {
                    let idx = self.constant(i);
                    let elem = self.get_index(scrut_val, idx, AccessKind::Direct);
                    self.test_pattern(sub, elem, fail_block)?;
                }
                Some(())
            }
            PatKind::Variant(path) => {
                let layout = self.layout_adt(path);
                let tag = self.is_instance(scrut_val, layout);
                self.jump_if_false(tag, fail_block);
                Some(())
            }
            PatKind::TupleVariant(path, sub_pats) => {
                let layout = self.layout_adt(path);
                let tag = self.is_instance(scrut_val, layout);
                self.jump_if_false(tag, fail_block);
                let ok_block = self.ir.push_block("pat_tag_ok");
                self.jump(ok_block);
                self.ir.target(ok_block);
                for (i, sub) in sub_pats.iter().enumerate() {
                    let elem = self.get_field(scrut_val, i as u32, AccessKind::Direct);
                    self.test_pattern(sub, elem, fail_block)?;
                }
                Some(())
            }
            PatKind::Struct(path, field_pats) => {
                let layout = self.layout_adt(path);
                let tag = self.is_instance(scrut_val, layout);
                self.jump_if_false(tag, fail_block);
                let ok_block = self.ir.push_block("pat_tag_ok");
                self.jump(ok_block);
                self.ir.target(ok_block);
                let field_order: Vec<String> = self.ir.resolutions.adts[layout].fields.clone();
                for (name, sub) in field_pats {
                    let slot = field_order
                        .iter()
                        .position(|f| f == name)
                        .expect("solver verified field exists")
                        as u32;
                    let elem = self.get_field(scrut_val, slot, AccessKind::Direct);
                    self.test_pattern(sub, elem, fail_block)?;
                }
                Some(())
            }
            PatKind::NullBind(inner) => {
                let cond = if matches!(
                    self.ir.resolutions.node_tys.get(&pat.id()),
                    Some(Ty::Result(_))
                ) {
                    let raised = self.is_raised(scrut_val);
                    self.unary(UnaryOp::Not, raised)
                } else {
                    let null = self.constant(Constant::Null);
                    self.bin(BinOp::NotEqual, scrut_val, null, OperandKind::Generic)
                };
                self.jump_if_false(cond, fail_block);
                let ok_block = self.ir.push_block("nullbind_ok");
                self.jump(ok_block);
                self.ir.target(ok_block);
                self.test_pattern(inner, scrut_val, fail_block)
            }
            PatKind::Or(alts) => {
                if alts.is_empty() {
                    self.jump(fail_block);
                    return Some(());
                }
                let success = self.ir.push_block("or_success");
                for (i, alt) in alts.iter().enumerate() {
                    let last = i + 1 == alts.len();
                    let try_next = if last {
                        fail_block
                    } else {
                        self.ir.push_block("or_try_next")
                    };
                    self.test_pattern(alt, scrut_val, try_next)?;
                    self.jump(success);
                    if !last {
                        self.ir.target(try_next);
                    }
                }
                self.ir.target(success);
                Some(())
            }
        }
    }

    pub(crate) fn bind_pattern(&mut self, pat: &Pat, scrut_val: InstId) {
        match pat.kind() {
            PatKind::Ident(_) => {
                let dec = self.ir.node_dec(pat.id());
                let local = self.ir.local_for(dec);
                self.set_local(local, scrut_val);
            }
            PatKind::Variant(_) => {}
            PatKind::TupleVariant(_, sub_pats) => {
                for (i, sub) in sub_pats.iter().enumerate() {
                    let elem = self.get_field(scrut_val, i as u32, AccessKind::Direct);
                    self.bind_pattern(sub, elem);
                }
            }
            PatKind::Struct(path, field_pats) => {
                let layout = self.layout_adt(path);
                let field_order: Vec<String> = self.ir.resolutions.adts[layout].fields.clone();
                for (name, sub) in field_pats {
                    let slot = field_order
                        .iter()
                        .position(|f| f == name)
                        .expect("solver verified field exists")
                        as u32;
                    let elem = self.get_field(scrut_val, slot, AccessKind::Direct);
                    self.bind_pattern(sub, elem);
                }
            }
            PatKind::Or(alts) => {
                if let Some(first) = alts.first() {
                    self.bind_pattern(first, scrut_val);
                }
            }
            PatKind::Literal(_) => {} // lits are okay! we use them in switches
            _ => unreachable!("bind_pattern on a refutable pattern; planner should exclude it"),
        }
    }

    /// Reads the layout adt id that the solver shadowed onto a pattern's path expression.
    fn layout_adt(&self, path: &parse::Expr) -> AdtId {
        match self.ir.resolutions.node_tys.get(&path.id()) {
            Some(Ty::Adt(adt) | Ty::Identity(adt)) => *adt,
            other => panic!("solver should have shadowed path with an adt type, got {other:?}",),
        }
    }

    pub(crate) fn id(&self) -> BlockId {
        self.ir.current_block_id()
    }

    pub(crate) fn constant(&mut self, constant: impl Into<Constant>) -> InstId {
        self.instruct(Inst::Constant(constant.into()))
    }

    pub(crate) fn set_local(&mut self, variable: Local, inst: InstId) -> InstId {
        self.instruct(Inst::SetLocal(variable, inst))
    }

    pub(crate) fn get_local(&mut self, variable: Local) -> InstId {
        self.instruct(Inst::GetLocal(variable))
    }

    pub(crate) fn bin(
        &mut self,
        op: BinOp,
        left: InstId,
        right: InstId,
        kind: OperandKind,
    ) -> InstId {
        self.instruct(Inst::BinOp {
            left,
            op,
            right,
            kind,
        })
    }

    pub(crate) fn unary(&mut self, op: UnaryOp, right: InstId) -> InstId {
        self.instruct(Inst::UnaryOp { op, right })
    }

    pub(crate) fn jump_if_false(&mut self, condition: InstId, target: BlockId) -> InstId {
        self.instruct(Inst::JumpIfFalse { condition, target })
    }

    pub(crate) fn jump_if_null(&mut self, value: InstId, target: BlockId) {
        let null = self.constant(Constant::Null);
        let non_null = self.bin(BinOp::NotEqual, value, null, OperandKind::Generic);
        self.jump_if_false(non_null, target);
    }

    pub(crate) fn jump(&mut self, target: BlockId) -> InstId {
        self.instruct(Inst::Jump { target })
    }

    pub(crate) fn for_next(&mut self, idx: InstId, bound: InstId, target: BlockId) -> InstId {
        self.instruct(Inst::ForNext { idx, bound, target })
    }

    pub(crate) fn switch(
        &mut self,
        scrut: InstId,
        base: u32,
        table: Vec<BlockId>,
        default: BlockId,
    ) -> InstId {
        self.instruct(Inst::Switch {
            scrut,
            base,
            table,
            default,
        })
    }

    pub(crate) fn phi(&mut self, branches: Vec<(BlockId, InstId)>) -> InstId {
        self.instruct(Inst::Phi(branches))
    }

    pub(crate) fn new_array(&mut self) -> InstId {
        self.instruct(Inst::NewArray)
    }

    pub(crate) fn new_dict(&mut self) -> InstId {
        self.instruct(Inst::NewDict)
    }

    pub(crate) fn push(&mut self, array: InstId, value: InstId) -> InstId {
        self.instruct(Inst::Push { array, value })
    }

    pub(crate) fn insert(&mut self, dict: InstId, key: StrId, value: InstId) -> InstId {
        self.instruct(Inst::Insert { dict, key, value })
    }

    pub(crate) fn set_index(&mut self, array: InstId, index: InstId, value: InstId) -> InstId {
        self.instruct(Inst::SetIndex {
            set: array,
            index,
            value,
        })
    }

    pub(crate) fn get_index(&mut self, set: InstId, index: InstId, kind: AccessKind) -> InstId {
        self.instruct(Inst::GetIndex { set, index, kind })
    }

    pub(crate) fn get_field(&mut self, src: InstId, slot: u32, kind: AccessKind) -> InstId {
        self.instruct(Inst::GetField { src, slot, kind })
    }

    pub(crate) fn set_field(&mut self, receiver: InstId, slot: u32, value: InstId) -> InstId {
        self.instruct(Inst::SetField {
            receiver,
            slot,
            value,
        })
    }

    pub(crate) fn len(&mut self, array: InstId) -> InstId {
        self.instruct(Inst::Len(array))
    }

    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn to_float(&mut self, value: InstId) -> InstId {
        self.instruct(Inst::ToFloat(value))
    }

    pub(crate) fn sqrt(&mut self, value: InstId) -> InstId {
        self.instruct(Inst::Sqrt(value))
    }

    pub(crate) fn unwrap(&mut self, value: InstId) -> InstId {
        self.instruct(Inst::Unwrap(value))
    }

    pub(crate) fn raise(&mut self, value: InstId) -> InstId {
        self.instruct(Inst::Raise(value))
    }

    pub(crate) fn is_raised(&mut self, value: InstId) -> InstId {
        self.instruct(Inst::IsRaised(value))
    }

    pub(crate) fn unwrap_raised(&mut self, value: InstId) -> InstId {
        self.instruct(Inst::UnwrapRaised(value))
    }

    pub(crate) fn check_in(&mut self, needle: InstId, target: InstId, condition: bool) -> InstId {
        self.instruct(Inst::In(needle, target, condition))
    }

    pub(crate) fn format(&mut self, parts: Vec<FormatPart>) -> InstId {
        self.instruct(Inst::Format(parts))
    }

    pub(crate) fn ref_body(&mut self, body: BodyId) -> InstId {
        self.instruct(Inst::RefBody(body))
    }

    pub(crate) fn make_closure(&mut self, body: BodyId, captures: Vec<InstId>) -> InstId {
        self.instruct(Inst::MakeClosure { body, captures })
    }

    pub(crate) fn call(&mut self, callee: InstId, args: Vec<InstId>) -> InstId {
        self.instruct(Inst::Call { callee, args })
    }

    pub(crate) fn call_direct(&mut self, body: BodyId, args: Vec<InstId>) -> InstId {
        self.instruct(Inst::CallDirect { body, args })
    }

    pub(crate) fn call_native(&mut self, id: NativeId, args: Vec<InstId>) -> InstId {
        self.instruct(Inst::CallNative { id, args })
    }

    pub(crate) fn ret(&mut self, value: InstId) -> InstId {
        self.instruct(Inst::Return(value))
    }

    pub(crate) fn new_instance(&mut self, adt: AdtId, fields: Vec<InstId>) -> InstId {
        self.instruct(Inst::NewInstance { adt, fields })
    }

    pub(crate) fn panic(&mut self) -> InstId {
        self.instruct(Inst::Panic)
    }

    pub(crate) fn is_instance(&mut self, src: InstId, adt: AdtId) -> InstId {
        self.instruct(Inst::IsInstance { src, adt })
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OperandKind {
    Bool,
    Int,
    Float,
    Str,
    Generic,
}
