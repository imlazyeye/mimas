use crate::{BinOp, BlockId, BodyId, Constant, Ctx, Encoder, Inst, OperandKind, Reg, UnaryOp};
use api::NativeId;
use parse::AccessKind;
use shared::StrId;
use solve::components::AdtId;

// i really hope you like macros

/// Reference table of every single Op and all of its data used by various macros below to define
/// each of their pieces
///
/// ## Format
/// OpName [ (prop, Ty, wire) <repeat for each prop> ];
///
/// Three important things to know:
///
/// 1. "wire" is a unique identifier used around the other macros to provide information
///
/// ... and then I apparently didn't write the other 2?
macro_rules! for_each_op {
    ($cb:ident) => {
        $cb! {
            // -- Placement -- //
            Move         [ (dst, Reg, reg) (src, Reg, reg) ];
            NewArray     [ (dst, Reg, reg) ];
            NewDict      [ (dst, Reg, reg) ];
            NewInstance  [ (dst, Reg, reg) (adt, AdtId, u32) (fields, Vec<Reg>, regs) ];
            NewClosure   [ (dst, Reg, reg) (body, BodyId, u32) (captures, Vec<Reg>, regs) ];
            Push         [ (array, Reg, reg) (value, Reg, reg) ];
            Insert       [ (dict, Reg, reg) (key, StrId, u32) (value, Reg, reg) ];
            SetIndex     [ (set, Reg, reg) (index, Reg, reg) (value, Reg, reg) ];
            SetField     [ (receiver, Reg, reg) (slot, u32, u32) (value, Reg, reg) ];
            LoadBody     [ (dst, Reg, reg) (body, BodyId, u32) ];
            LoadConst    [ (dst, Reg, reg) (constant, Constant, konst) ];

            // -- Control Flow -- //
            Jump         [ (target, BlockTarget, jump) ];
            JumpIf       [ (cond, Reg, reg) (target, BlockTarget, jump) (is_true, bool, bool) ];
            ForNext      [ (idx, Reg, reg) (bound, Reg, reg) (target, BlockTarget, jump) ];
            Switch       [ (scrut, Reg, reg) (base, u32, u32) (default, BlockTarget, jump) (table, Vec<BlockTarget>, jumptable) ];
            Call         [ (dst, Reg, reg) (callee, Reg, reg) (args, Vec<Reg>, regs) ];
            CallDirect   [ (dst, Reg, reg) (body, BodyId, u32) (args, Vec<Reg>, regs) ];
            CallNative   [ (dst, Reg, reg) (id, NativeId, u32) (args, Vec<Reg>, regs) ];
            Return       [ (val, Reg, reg) ];

            // -- Errors/Termination -- //
            Panic        [ ];
            Raise        [ (val, Reg, reg) ];
            IsRaised     [ (dst, Reg, reg) (src, Reg, reg) ];
            UnwrapRaised [ (dst, Reg, reg) (src, Reg, reg) ];
            Unwrap       [ (dst, Reg, reg) (src, Reg, reg) ];

            // -- Inquires -- //
            GetIndex     [ (dst, Reg, reg) (set, Reg, reg) (index, Reg, reg) (kind, AccessKind, enum8) ];
            GetField     [ (dst, Reg, reg) (src, Reg, reg) (slot, u32, u32) (kind, AccessKind, enum8) ];
            Len          [ (dst, Reg, reg) (src, Reg, reg) ];
            ToFloat      [ (dst, Reg, reg) (src, Reg, reg) ];
            Sqrt         [ (dst, Reg, reg) (src, Reg, reg) ];
            In           [ (dst, Reg, reg) (needle, Reg, reg) (haystack, Reg, reg) (condition, bool, bool) ];
            IsInstance   [ (dst, Reg, reg) (src, Reg, reg) (adt, AdtId, u32) ];

            // -- Misc -- //
            Format       [ (dst, Reg, reg) (parts, Vec<OpFormatPart>, fparts) ];

            // -- Non-Specialized Evaluation -- //
            Bin          [ (dst, Reg, reg) (left, Reg, reg) (op, BinOp, enum8) (right, Reg, reg) ];
            Unary        [ (dst, Reg, reg) (op, UnaryOp, enum8) (src, Reg, reg) ];

            // -- Bool Specialized --//
            BoolEq       [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            BoolNe       [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];

            // -- Integer Specialized -- //
            AddInt       [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            SubInt       [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            MultInt      [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            ModInt       [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            IntLt        [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            IntLe        [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            IntGt        [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            IntGe        [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            IntEq        [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            IntNe        [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            AddIntImm    [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            SubIntImm    [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            MultIntImm   [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            ModIntImm    [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            IntLtImm     [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            IntLeImm     [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            IntGtImm     [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            IntGeImm     [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            IntEqImm     [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            IntNeImm     [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            BIntLt       [ (target, BlockTarget, jump) (left, Reg, reg) (right, Reg, reg) (is_true, bool, bool) ];
            BIntLe       [ (target, BlockTarget, jump) (left, Reg, reg) (right, Reg, reg) (is_true, bool, bool) ];
            BIntGt       [ (target, BlockTarget, jump) (left, Reg, reg) (right, Reg, reg) (is_true, bool, bool) ];
            BIntGe       [ (target, BlockTarget, jump) (left, Reg, reg) (right, Reg, reg) (is_true, bool, bool) ];
            BIntEq       [ (target, BlockTarget, jump) (left, Reg, reg) (right, Reg, reg) (is_true, bool, bool) ];
            BIntNe       [ (target, BlockTarget, jump) (left, Reg, reg) (right, Reg, reg) (is_true, bool, bool) ];
            BIntLtImm    [ (target, BlockTarget, jump) (left, Reg, reg) (val, i64, i64) (is_true, bool, bool) ];
            BIntLeImm    [ (target, BlockTarget, jump) (left, Reg, reg) (val, i64, i64) (is_true, bool, bool) ];
            BIntGtImm    [ (target, BlockTarget, jump) (left, Reg, reg) (val, i64, i64) (is_true, bool, bool) ];
            BIntGeImm    [ (target, BlockTarget, jump) (left, Reg, reg) (val, i64, i64) (is_true, bool, bool) ];
            BIntEqImm    [ (target, BlockTarget, jump) (left, Reg, reg) (val, i64, i64) (is_true, bool, bool) ];
            BIntNeImm    [ (target, BlockTarget, jump) (left, Reg, reg) (val, i64, i64) (is_true, bool, bool) ];

            // -- Float Specialized -- //
            AddFloat     [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            SubFloat     [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            MultFloat    [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            DivFloat     [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            FloatLt      [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            FloatLe      [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            FloatGt      [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            FloatGe      [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            FloatEq      [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            FloatNe      [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            AddFloatImm  [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            SubFloatImm  [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            MultFloatImm [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            ModFloatImm  [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            FloatLtImm   [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            FloatLeImm   [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            FloatGtImm   [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            FloatGeImm   [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            FloatEqImm   [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            FloatNeImm   [ (dst, Reg, reg) (left, Reg, reg) (val, i64, i64) ];
            BFloatLt     [ (target, BlockTarget, jump) (left, Reg, reg) (right, Reg, reg) (is_true, bool, bool) ];
            BFloatLe     [ (target, BlockTarget, jump) (left, Reg, reg) (right, Reg, reg) (is_true, bool, bool) ];
            BFloatGt     [ (target, BlockTarget, jump) (left, Reg, reg) (right, Reg, reg) (is_true, bool, bool) ];
            BFloatGe     [ (target, BlockTarget, jump) (left, Reg, reg) (right, Reg, reg) (is_true, bool, bool) ];
            BFloatEq     [ (target, BlockTarget, jump) (left, Reg, reg) (right, Reg, reg) (is_true, bool, bool) ];
            BFloatNe     [ (target, BlockTarget, jump) (left, Reg, reg) (right, Reg, reg) (is_true, bool, bool) ];
            BFloatLtImm  [ (target, BlockTarget, jump) (left, Reg, reg) (val, i64, i64) (is_true, bool, bool) ];
            BFloatLeImm  [ (target, BlockTarget, jump) (left, Reg, reg) (val, i64, i64) (is_true, bool, bool) ];
            BFloatGtImm  [ (target, BlockTarget, jump) (left, Reg, reg) (val, i64, i64) (is_true, bool, bool) ];
            BFloatGeImm  [ (target, BlockTarget, jump) (left, Reg, reg) (val, i64, i64) (is_true, bool, bool) ];
            BFloatEqImm  [ (target, BlockTarget, jump) (left, Reg, reg) (val, i64, i64) (is_true, bool, bool) ];
            BFloatNeImm  [ (target, BlockTarget, jump) (left, Reg, reg) (val, i64, i64) (is_true, bool, bool) ];

            // -- String Specialized -- //
            StrEq        [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
            StrNe        [ (dst, Reg, reg) (left, Reg, reg) (right, Reg, reg) ];
        }
    };
}

macro_rules! define_op {
    ( $( $name:ident [ $( ($f:ident, $ty:ty, $w:ident) )* ]; )* ) => {
        #[repr(u8)]
        #[derive(Debug, Clone)]
        pub(crate) enum Op {
            $( $name { $( $f: $ty ),* }, )*
        }
    };
}
for_each_op!(define_op);

macro_rules! define_opcode {
    ( $( $name:ident [ $($_fields:tt)* ]; )* ) => {
        #[repr(u8)]
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum OpCode { $( $name, )* }
        impl OpCode {
            pub const COUNT: usize = [ $( OpCode::$name ),* ].len();
        }
    };
}
for_each_op!(define_opcode);

macro_rules! define_encode {
    ( $( $name:ident [ $( ($f:ident, $ty:ty, $w:ident) )* ]; )* ) => {
        pub(crate) fn encode(self, e: &mut Encoder) {
            e.u8(self.discriminant());
            match self {
                $( Op::$name { $( $f ),* } => { $( encode_field!(e, $w, $f); )* } )*
            }
        }
    };
}

// per-field encoding, keyed on wire kind. each arm expands in statement position (not as a whole
// match arm), so the expansion stays well-formed.
macro_rules! encode_field {
    ($e:ident, reg,    $v:ident) => {
        $e.reg($v);
    };
    ($e:ident, u32,    $v:ident) => {
        $e.u32($v);
    };
    ($e:ident, i64,    $v:ident) => {
        $e.i64($v);
    };
    ($e:ident, enum8,  $v:ident) => {
        $e.u8($v as u8);
    };
    ($e:ident, regs,   $v:ident) => {
        $e.u8($v.len() as u8);
        for x in $v {
            $e.reg(x);
        }
    };
    ($e:ident, jump,   $v:ident) => {
        let BlockTarget::ByteOffset(off) = $v else {
            unreachable!("unresolved BlockTarget!")
        };
        $e.u32(u32::try_from(off).unwrap());
    };
    ($e:ident, konst,  $v:ident) => {
        $e.u8(crate::ir::constant_discriminant(&$v));
        $v.encode($e);
    };
    ($e:ident, fparts, $v:ident) => {
        $e.u16(u16::try_from($v.len()).expect("> 65535 fstring parts"));
        for p in $v {
            $e.u8(format_part_discriminant(&p));
            match p {
                OpFormatPart::Literal(s) => $e.u32(s),
                OpFormatPart::Value(r) => $e.reg(r),
            }
        }
    };
    ($e:ident, bool, $v:ident) => {
        $e.u8($v as u8)
    };
    ($e:ident, jumptable, $v:ident) => {
        $e.u16(u16::try_from($v.len()).expect("> 65535 switch arms, congrats, you iced us"));
        for t in $v {
            let BlockTarget::ByteOffset(off) = t else {
                unreachable!("unresolved BlockTarget")
            };
            $e.u32(u32::try_from(off).unwrap());
        }
    };
}

macro_rules! define_encoded_len {
    ( $( $name:ident [ $( ($f:ident, $ty:ty, $w:ident) )* ]; )* ) => {
        pub fn encoded_len(&self) -> usize {
            1 + match self {
                $( Op::$name { $( $f ),* } => 0 $( + len_field!($w, $f) )* , )*
            }
        }
    };
}

macro_rules! len_field {
    (reg,    $f:ident) => {{
        let _ = $f;
        2
    }};
    (u32,    $f:ident) => {{
        let _ = $f;
        4
    }};
    (i64,    $f:ident) => {{
        let _ = $f;
        8
    }};
    (enum8,  $f:ident) => {{
        let _ = $f;
        1
    }};
    (jump,   $f:ident) => {{
        let _ = $f;
        4
    }};
    (regs,   $f:ident) => {
        (1 + $f.len() * 2)
    };
    (konst,  $f:ident) => {
        (1 + $f.byte_len())
    };
    (fparts, $f:ident) => {
        (2 + $f
            .iter()
            .map(|p| match p {
                OpFormatPart::Literal(_) => 5,
                OpFormatPart::Value(_) => 3,
            })
            .sum::<usize>())
    };
    (bool, $f:ident) => {{
        let _ = $f;
        1
    }};
    (jumptable, $f:ident) => {
        (2 + $f.len() * 4)
    };
}

// Munches each variant to find its `dst` Reg -- this relies on our invariant mentioned in
// for_each_op above!
macro_rules! op_dst_match {
    // base case: every op consumed -- emit the assembled match.
    (
        $op:ident { $($arms:tt)* }
    ) => {
        match $op { $($arms)* }
    };

    // peel an op whose first field is `dst`: bind it and yield it by reference.
    (
        $op:ident { $($arms:tt)* }
        $name:ident [ (dst, $($_dst:tt)*) $($_fields:tt)* ]
        $($rest:tt)*
    ) => {
        op_dst_match!($op {
            $($arms)*
            Op::$name { dst, .. } => Some(dst),
        } $($rest)*)
    };

    // peel any other op: it produces no register.
    (
        $op:ident { $($arms:tt)* }
        $name:ident [ $($_fields:tt)* ]
        $($rest:tt)*
    ) => {
        op_dst_match!($op {
            $($arms)*
            Op::$name { .. } => None,
        } $($rest)*)
    };
}

macro_rules! define_dst_accessors {
    ( $( $name:ident [ $($fields:tt)* ]; )* ) => {
        /// The register this op writes to, or `None` if it produces no value.
        pub fn reg(&self) -> Option<Reg> {
            let op = self;
            op_dst_match!(op { } $( $name [ $($fields)* ] )* ).copied()
        }
        /// Rewrite this op's destination register; no-op for ops that produce no value.
        pub fn set_reg(&mut self, dst: Reg) {
            let op = self;
            if let Some(r) = op_dst_match!(op { } $( $name [ $($fields)* ] )* ) {
                *r = dst;
            }
        }
        /// Whether or not this is a comparison op.
        pub fn is_comparison(&self) -> bool {
            match self {
                Op::IntLt { .. }
                | Op::IntLe { .. }
                | Op::IntGt { .. }
                | Op::IntGe { .. }
                | Op::IntEq { .. }
                | Op::IntNe { .. }
                | Op::FloatLt { .. }
                | Op::FloatLe { .. }
                | Op::FloatGt { .. }
                | Op::FloatGe { .. }
                | Op::FloatEq { .. }
                | Op::FloatNe { .. }
                | Op::IntLtImm { .. }
                | Op::IntLeImm { .. }
                | Op::IntGtImm { .. }
                | Op::IntGeImm { .. }
                | Op::IntEqImm { .. }
                | Op::IntNeImm { .. }
                | Op::FloatLtImm { .. }
                | Op::FloatLeImm { .. }
                | Op::FloatGtImm { .. }
                | Op::FloatGeImm { .. }
                | Op::FloatEqImm { .. }
                | Op::FloatNeImm { .. } => true,
                _ => false,
            }
        }
    };
}

// pick the specialized typed Op for each (BinOp, OperandKind); anything unlisted falls back to
// Op::Bin.
macro_rules! typed_binop {
    ($op:expr, $kind:expr, $dst:expr, $left:expr, $right:expr;
     $(($bop:ident, $k:ident) => $variant:ident),+ $(,)?) => {
        match ($op, $kind) {
            $((BinOp::$bop, OperandKind::$k) => Op::$variant { dst: $dst, left: $left, right: $right },)+
            (op, _) => Op::Bin { dst: $dst, left: $left, op: *op, right: $right },
        }
    };
}

impl Op {
    for_each_op!(define_encode);
    for_each_op!(define_encoded_len);
    for_each_op!(define_dst_accessors);

    // discriminant lives in the first byte of a repr(u8) enum -- fixed fact, not table-driven.
    #[inline(always)]
    pub fn discriminant(&self) -> u8 {
        unsafe { *(self as *const Self as *const u8) }
    }

    pub fn from_inst(inst: &Inst, mut ctx: Ctx<'_>) -> Self {
        match inst {
            Inst::Constant(constant) => Op::LoadConst {
                dst: ctx.reg(),
                constant: constant.clone(),
            },
            Inst::SetLocal(local, value) => Op::Move {
                dst: ctx.local_to_reg[*local],
                src: ctx.i2r(value),
            },
            Inst::GetLocal(_) => unreachable!("GetLocal is aliased at codegen, never emitted"),
            Inst::Phi(_) => unreachable!("phis should be handled before Op::from_inst"),
            Inst::BinOp {
                left,
                op,
                right,
                kind,
            } => {
                let dst = ctx.reg();
                let left = ctx.i2r(left);
                let right = ctx.i2r(right);
                typed_binop!(op, kind, dst, left, right;
                    (Identity, Bool) => BoolEq, (NotEqual, Bool) => BoolNe,
                    (Add, Int) => AddInt, (Sub, Int) => SubInt, (Mult, Int) => MultInt,
                    (Mod, Int) => ModInt,
                    (LessThan, Int) => IntLt, (LessEqual, Int) => IntLe,
                    (GreaterThan, Int) => IntGt, (GreaterEqual, Int) => IntGe,
                    (Identity, Int) => IntEq, (NotEqual, Int) => IntNe,
                    (Add, Float) => AddFloat, (Sub, Float) => SubFloat, (Mult, Float) => MultFloat,
                    (Div, Float) => DivFloat,
                    (LessThan, Float) => FloatLt, (LessEqual, Float) => FloatLe,
                    (GreaterThan, Float) => FloatGt, (GreaterEqual, Float) => FloatGe,
                    (Identity, Float) => FloatEq, (NotEqual, Float) => FloatNe,
                    (Identity, Str) => StrEq, (NotEqual, Str) => StrNe,
                )
            }
            Inst::UnaryOp { op, right } => Op::Unary {
                dst: ctx.reg(),
                op: *op,
                src: ctx.i2r(right),
            },
            Inst::JumpIfFalse { condition, target } => Op::JumpIf {
                cond: ctx.i2r(condition),
                target: BlockTarget::Block(*target),
                is_true: false,
            },
            Inst::Jump { target } => Op::Jump {
                target: BlockTarget::Block(*target),
            },
            Inst::Switch {
                scrut,
                base,
                table,
                default,
            } => Op::Switch {
                scrut: ctx.i2r(scrut),
                base: *base,
                default: BlockTarget::Block(*default),
                table: table.iter().map(|b| BlockTarget::Block(*b)).collect(),
            },
            Inst::NewArray => Op::NewArray { dst: ctx.reg() },
            Inst::NewDict => Op::NewDict { dst: ctx.reg() },
            Inst::GetIndex { set, index, kind } => Op::GetIndex {
                dst: ctx.reg(),
                set: ctx.i2r(set),
                index: ctx.i2r(index),
                kind: *kind,
            },
            Inst::SetIndex { set, index, value } => Op::SetIndex {
                set: ctx.i2r(set),
                index: ctx.i2r(index),
                value: ctx.i2r(value),
            },
            Inst::Push { array, value } => Op::Push {
                array: ctx.i2r(array),
                value: ctx.i2r(value),
            },
            Inst::Insert { dict, key, value } => Op::Insert {
                dict: ctx.i2r(dict),
                key: *key,
                value: ctx.i2r(value),
            },
            Inst::Len(value) => Op::Len {
                dst: ctx.reg(),
                src: ctx.i2r(value),
            },
            Inst::ToFloat(value) => Op::ToFloat {
                dst: ctx.reg(),
                src: ctx.i2r(value),
            },
            Inst::Sqrt(value) => Op::Sqrt {
                dst: ctx.reg(),
                src: ctx.i2r(value),
            },
            Inst::Unwrap(value) => Op::Unwrap {
                dst: ctx.reg(),
                src: ctx.i2r(value),
            },
            Inst::In(needle, haystack, condition) => Op::In {
                dst: ctx.reg(),
                needle: ctx.i2r(needle),
                haystack: ctx.i2r(haystack),
                condition: *condition,
            },
            Inst::ForNext { idx, bound, target } => Op::ForNext {
                idx: ctx.i2r(idx),
                bound: ctx.i2r(bound),
                target: BlockTarget::Block(*target),
            },
            Inst::Format(parts) => Op::Format {
                dst: ctx.reg(),
                parts: parts
                    .iter()
                    .map(|part| match part {
                        crate::FormatPart::Literal(s) => OpFormatPart::Literal(*s),
                        crate::FormatPart::Value(value) => OpFormatPart::Value(ctx.i2r(value)),
                    })
                    .collect(),
            },
            Inst::RefBody(body) => Op::LoadBody {
                dst: ctx.reg(),
                body: *body,
            },
            Inst::MakeClosure { body, captures } => Op::NewClosure {
                dst: ctx.reg(),
                body: *body,
                captures: captures.iter().map(|c| ctx.i2r(c)).collect(),
            },
            Inst::Call { callee, args } => Op::Call {
                dst: ctx.reg(),
                callee: ctx.i2r(callee),
                args: args.iter().map(|arg| ctx.i2r(arg)).collect(),
            },
            Inst::CallDirect { body, args } => Op::CallDirect {
                dst: ctx.reg(),
                body: *body,
                args: args.iter().map(|arg| ctx.i2r(arg)).collect(),
            },
            Inst::GetField { src, slot, kind } => Op::GetField {
                dst: ctx.reg(),
                src: ctx.i2r(src),
                slot: *slot,
                kind: *kind,
            },
            Inst::SetField {
                receiver,
                slot,
                value,
            } => Op::SetField {
                receiver: ctx.i2r(receiver),
                slot: *slot,
                value: ctx.i2r(value),
            },
            Inst::Return(value) => Op::Return {
                val: ctx.i2r(value),
            },
            Inst::NewInstance { adt, fields } => Op::NewInstance {
                dst: ctx.reg(),
                adt: *adt,
                fields: fields.iter().map(|field| ctx.i2r(field)).collect(),
            },
            Inst::CallNative { id, args } => Op::CallNative {
                dst: ctx.reg(),
                id: *id,
                args: args.iter().map(|arg| ctx.i2r(arg)).collect(),
            },
            Inst::Panic => Op::Panic {},
            Inst::IsInstance { src, adt } => Op::IsInstance {
                dst: ctx.reg(),
                src: ctx.i2r(src),
                adt: *adt,
            },
            Inst::Raise(value) => Op::Raise {
                val: ctx.i2r(value),
            },
            Inst::IsRaised(value) => Op::IsRaised {
                dst: ctx.reg(),
                src: ctx.i2r(value),
            },
            Inst::UnwrapRaised(value) => Op::UnwrapRaised {
                dst: ctx.reg(),
                src: ctx.i2r(value),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum BlockTarget {
    Block(BlockId),
    ByteOffset(usize),
}

// discriminants here are the on-wire tag -- keep them in sync with OpFormatPartCode.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum OpFormatPart {
    Literal(StrId) = 0,
    Value(Reg) = 1,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpFormatPartCode {
    Literal = 0,
    Value = 1,
}

#[inline(always)]
fn format_part_discriminant(p: &OpFormatPart) -> u8 {
    unsafe { *(p as *const OpFormatPart as *const u8) }
}
