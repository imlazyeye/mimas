use std::collections::HashSet;

use super::super::{
    BinOp, BlockTarget, BodyId, Constant, Decode, Decoder, Encoder, Op, OpCode, OpFormatPart, Reg,
    UnaryOp,
};
use api::NativeId;
use parse::AccessKind;
use shared::StrId;
use solve::components::AdtId;

use super::compile_test_utils::compile_ops;

// part 1: encode -> decode -> re-encode round-trip

fn encode_bytes(op: &Op) -> Vec<u8> {
    let mut e = Encoder::new();
    op.clone().encode(&mut e);
    e.finish()
}

// decode every field of the op that starts at `decoder.ip`, mirroring `encode`'s field order
// exactly, and rebuild an equivalent `Op`. field decoders are the same `Decode` impls the VM
// dispatch uses, so this exercises the real decode path. i64/regs/jumptable/fparts have no `Decode`
// impl (they're length-prefixed or primitive) so we read them directly off the decoder.
fn decode_op(d: &mut Decoder) -> Op {
    let regs = |d: &mut Decoder| -> Vec<Reg> {
        let n = d.u8() as usize;
        (0..n).map(|_| Reg::decode(d)).collect()
    };
    match OpCode::decode(d) {
        OpCode::Move => Op::Move {
            dst: Reg::decode(d),
            src: Reg::decode(d),
        },
        OpCode::NewArray => Op::NewArray {
            dst: Reg::decode(d),
        },
        OpCode::NewDict => Op::NewDict {
            dst: Reg::decode(d),
        },
        OpCode::NewInstance => Op::NewInstance {
            dst: Reg::decode(d),
            adt: AdtId::decode(d),
            fields: regs(d),
        },
        OpCode::NewClosure => Op::NewClosure {
            dst: Reg::decode(d),
            body: BodyId::decode(d),
            captures: regs(d),
        },
        OpCode::Push => Op::Push {
            array: Reg::decode(d),
            value: Reg::decode(d),
        },
        OpCode::Insert => Op::Insert {
            dict: Reg::decode(d),
            key: StrId::from(d.u32()),
            value: Reg::decode(d),
        },
        OpCode::SetIndex => Op::SetIndex {
            set: Reg::decode(d),
            index: Reg::decode(d),
            value: Reg::decode(d),
        },
        OpCode::SetField => Op::SetField {
            receiver: Reg::decode(d),
            slot: d.u32(),
            value: Reg::decode(d),
        },
        OpCode::LoadBody => Op::LoadBody {
            dst: Reg::decode(d),
            body: BodyId::decode(d),
        },
        OpCode::LoadConst => Op::LoadConst {
            dst: Reg::decode(d),
            constant: Constant::decode(d),
        },
        OpCode::Jump => Op::Jump {
            target: BlockTarget::decode(d),
        },
        OpCode::JumpIf => Op::JumpIf {
            cond: Reg::decode(d),
            target: BlockTarget::decode(d),
            is_true: bool::decode(d),
        },
        OpCode::ForNext => Op::ForNext {
            idx: Reg::decode(d),
            bound: Reg::decode(d),
            target: BlockTarget::decode(d),
        },
        OpCode::Switch => {
            let scrut = Reg::decode(d);
            let base = d.u32();
            let default = BlockTarget::decode(d);
            let n = d.u16() as usize;
            let table = (0..n).map(|_| BlockTarget::decode(d)).collect();
            Op::Switch {
                scrut,
                base,
                default,
                table,
            }
        }
        OpCode::Call => Op::Call {
            dst: Reg::decode(d),
            callee: Reg::decode(d),
            args: regs(d),
        },
        OpCode::CallDirect => Op::CallDirect {
            dst: Reg::decode(d),
            body: BodyId::decode(d),
            args: regs(d),
        },
        OpCode::CallNative => Op::CallNative {
            dst: Reg::decode(d),
            id: NativeId::decode(d),
            args: regs(d),
        },
        OpCode::Return => Op::Return {
            val: Reg::decode(d),
        },
        OpCode::Panic => Op::Panic {},
        OpCode::Raise => Op::Raise {
            val: Reg::decode(d),
        },
        OpCode::IsRaised => Op::IsRaised {
            dst: Reg::decode(d),
            src: Reg::decode(d),
        },
        OpCode::UnwrapRaised => Op::UnwrapRaised {
            dst: Reg::decode(d),
            src: Reg::decode(d),
        },
        OpCode::Unwrap => Op::Unwrap {
            dst: Reg::decode(d),
            src: Reg::decode(d),
        },
        OpCode::GetIndex => Op::GetIndex {
            dst: Reg::decode(d),
            set: Reg::decode(d),
            index: Reg::decode(d),
            kind: AccessKind::decode(d),
        },
        OpCode::GetField => Op::GetField {
            dst: Reg::decode(d),
            src: Reg::decode(d),
            slot: d.u32(),
            kind: AccessKind::decode(d),
        },
        OpCode::Len => Op::Len {
            dst: Reg::decode(d),
            src: Reg::decode(d),
        },
        OpCode::ToFloat => Op::ToFloat {
            dst: Reg::decode(d),
            src: Reg::decode(d),
        },
        OpCode::Sqrt => Op::Sqrt {
            dst: Reg::decode(d),
            src: Reg::decode(d),
        },
        OpCode::In => Op::In {
            dst: Reg::decode(d),
            needle: Reg::decode(d),
            haystack: Reg::decode(d),
            condition: bool::decode(d),
        },
        OpCode::IsInstance => Op::IsInstance {
            dst: Reg::decode(d),
            src: Reg::decode(d),
            adt: AdtId::decode(d),
        },
        OpCode::Format => {
            let dst = Reg::decode(d);
            let n = d.u16() as usize;
            let parts = (0..n).map(|_| OpFormatPart::decode(d)).collect();
            Op::Format { dst, parts }
        }
        OpCode::Bin => Op::Bin {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            op: BinOp::decode(d),
            right: Reg::decode(d),
        },
        OpCode::Unary => Op::Unary {
            dst: Reg::decode(d),
            op: UnaryOp::decode(d),
            src: Reg::decode(d),
        },
        OpCode::BoolEq => Op::BoolEq {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::BoolNe => Op::BoolNe {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::AddInt => Op::AddInt {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::SubInt => Op::SubInt {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::MultInt => Op::MultInt {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::ModInt => Op::ModInt {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::IntLt => Op::IntLt {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::IntLe => Op::IntLe {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::IntGt => Op::IntGt {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::IntGe => Op::IntGe {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::IntEq => Op::IntEq {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::IntNe => Op::IntNe {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::AddIntImm => Op::AddIntImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::SubIntImm => Op::SubIntImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::MultIntImm => Op::MultIntImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::ModIntImm => Op::ModIntImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::IntLtImm => Op::IntLtImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::IntLeImm => Op::IntLeImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::IntGtImm => Op::IntGtImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::IntGeImm => Op::IntGeImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::IntEqImm => Op::IntEqImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::IntNeImm => Op::IntNeImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::BIntLt => Op::BIntLt {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
            is_true: bool::decode(d),
        },
        OpCode::BIntLe => Op::BIntLe {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
            is_true: bool::decode(d),
        },
        OpCode::BIntGt => Op::BIntGt {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
            is_true: bool::decode(d),
        },
        OpCode::BIntGe => Op::BIntGe {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
            is_true: bool::decode(d),
        },
        OpCode::BIntEq => Op::BIntEq {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
            is_true: bool::decode(d),
        },
        OpCode::BIntNe => Op::BIntNe {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
            is_true: bool::decode(d),
        },
        OpCode::BIntLtImm => Op::BIntLtImm {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
            is_true: bool::decode(d),
        },
        OpCode::BIntLeImm => Op::BIntLeImm {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
            is_true: bool::decode(d),
        },
        OpCode::BIntGtImm => Op::BIntGtImm {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
            is_true: bool::decode(d),
        },
        OpCode::BIntGeImm => Op::BIntGeImm {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
            is_true: bool::decode(d),
        },
        OpCode::BIntEqImm => Op::BIntEqImm {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
            is_true: bool::decode(d),
        },
        OpCode::BIntNeImm => Op::BIntNeImm {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
            is_true: bool::decode(d),
        },
        OpCode::AddFloat => Op::AddFloat {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::SubFloat => Op::SubFloat {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::MultFloat => Op::MultFloat {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::DivFloat => Op::DivFloat {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::FloatLt => Op::FloatLt {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::FloatLe => Op::FloatLe {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::FloatGt => Op::FloatGt {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::FloatGe => Op::FloatGe {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::FloatEq => Op::FloatEq {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::FloatNe => Op::FloatNe {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::BFloatLt => Op::BFloatLt {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
            is_true: bool::decode(d),
        },
        OpCode::BFloatLe => Op::BFloatLe {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
            is_true: bool::decode(d),
        },
        OpCode::BFloatGt => Op::BFloatGt {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
            is_true: bool::decode(d),
        },
        OpCode::BFloatGe => Op::BFloatGe {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
            is_true: bool::decode(d),
        },
        OpCode::BFloatEq => Op::BFloatEq {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
            is_true: bool::decode(d),
        },
        OpCode::BFloatNe => Op::BFloatNe {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
            is_true: bool::decode(d),
        },
        OpCode::AddFloatImm => Op::AddFloatImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::SubFloatImm => Op::SubFloatImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::MultFloatImm => Op::MultFloatImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::ModFloatImm => Op::ModFloatImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::FloatLtImm => Op::FloatLtImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::FloatLeImm => Op::FloatLeImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::FloatGtImm => Op::FloatGtImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::FloatGeImm => Op::FloatGeImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::FloatEqImm => Op::FloatEqImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::FloatNeImm => Op::FloatNeImm {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
        },
        OpCode::BFloatLtImm => Op::BFloatLtImm {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
            is_true: bool::decode(d),
        },
        OpCode::BFloatLeImm => Op::BFloatLeImm {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
            is_true: bool::decode(d),
        },
        OpCode::BFloatGtImm => Op::BFloatGtImm {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
            is_true: bool::decode(d),
        },
        OpCode::BFloatGeImm => Op::BFloatGeImm {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
            is_true: bool::decode(d),
        },
        OpCode::BFloatEqImm => Op::BFloatEqImm {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
            is_true: bool::decode(d),
        },
        OpCode::BFloatNeImm => Op::BFloatNeImm {
            target: BlockTarget::decode(d),
            left: Reg::decode(d),
            val: d.i64(),
            is_true: bool::decode(d),
        },
        OpCode::StrEq => Op::StrEq {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
        OpCode::StrNe => Op::StrNe {
            dst: Reg::decode(d),
            left: Reg::decode(d),
            right: Reg::decode(d),
        },
    }
}

// encode op -> decode it back -> re-encode the decoded op. asserts:
//   - encoded_len() == bytes actually written,
//   - decode consumed exactly that byte count, and
//   - the decoded op re-encodes to byte-identical output (no field corruption/reordering).
// returns the discriminant so callers can confirm full opcode coverage.
fn roundtrip(op: Op) -> u8 {
    let code = op.discriminant();
    let bytes = encode_bytes(&op);
    assert_eq!(
        bytes.len(),
        op.encoded_len(),
        "encoded_len != bytes written for {op:?}"
    );

    let mut d = Decoder {
        bytes: bytes.clone(),
        ip: 0,
    };
    let decoded = decode_op(&mut d);
    assert_eq!(
        d.ip,
        bytes.len(),
        "decoder consumed {} of {} bytes for {op:?}",
        d.ip,
        bytes.len()
    );

    let re = encode_bytes(&decoded);
    assert_eq!(
        re, bytes,
        "decode->re-encode mismatch\n original: {op:?}\n decoded:  {decoded:?}"
    );
    code
}

fn r(n: u32) -> Reg {
    Reg::from(n)
}

// a representative value for each field type that appears in `for_each_op!`. distinct nonzero
// values so a swapped/dropped field shows up in the re-encode diff. Vec fields are non-empty so the
// length prefix gets exercised; the empty-vec and large-vec extremes get dedicated tests below.
trait Sample {
    fn sample() -> Self;
}
impl Sample for Reg {
    fn sample() -> Self {
        r(2)
    }
}
impl Sample for u32 {
    fn sample() -> Self {
        7
    }
}
impl Sample for i64 {
    fn sample() -> Self {
        -5
    }
}
impl Sample for bool {
    fn sample() -> Self {
        true
    }
}
impl Sample for AdtId {
    fn sample() -> Self {
        AdtId::from(11)
    }
}
impl Sample for BodyId {
    fn sample() -> Self {
        BodyId::from(9)
    }
}
impl Sample for NativeId {
    fn sample() -> Self {
        NativeId::from(13)
    }
}
impl Sample for StrId {
    fn sample() -> Self {
        StrId::from(7)
    }
}
impl Sample for AccessKind {
    fn sample() -> Self {
        AccessKind::Option
    }
}
impl Sample for BinOp {
    fn sample() -> Self {
        BinOp::Div
    }
}
impl Sample for UnaryOp {
    fn sample() -> Self {
        UnaryOp::Not
    }
}
impl Sample for Constant {
    fn sample() -> Self {
        Constant::Int(-9223372036854775808)
    }
}
impl Sample for BlockTarget {
    fn sample() -> Self {
        BlockTarget::ByteOffset(123)
    }
}
impl Sample for Vec<Reg> {
    fn sample() -> Self {
        vec![r(3), r(4)]
    }
}
impl Sample for Vec<BlockTarget> {
    fn sample() -> Self {
        vec![BlockTarget::ByteOffset(10), BlockTarget::ByteOffset(20)]
    }
}
impl Sample for Vec<OpFormatPart> {
    fn sample() -> Self {
        vec![
            OpFormatPart::Literal(StrId::from(7)),
            OpFormatPart::Value(r(2)),
        ]
    }
}

// one representative `Op` per variant, generated straight off `for_each_op!` so the list can never
// fall out of sync with the real opcode set.
macro_rules! sample_ops {
    ( $( $name:ident [ $( ($f:ident, $ty:ty, $w:ident) )* ]; )* ) => {
        vec![ $( Op::$name { $( $f: <$ty as Sample>::sample() ),* }, )* ]
    };
}

fn all_ops() -> Vec<Op> {
    for_each_op!(sample_ops)
}

// every Op encodes/decodes/re-encodes byte-identically, and the table covers every opcode.
#[test]
fn roundtrip_covers_every_opcode() {
    let seen: HashSet<u8> = all_ops().into_iter().map(roundtrip).collect();
    assert_eq!(
        seen.len(),
        OpCode::COUNT,
        "roundtrip list missed {} opcode(s)",
        OpCode::COUNT - seen.len()
    );
}

// the discriminant of an Op must equal its OpCode's repr index -- both come off `for_each_op!` in
// the same order, but the unsafe `discriminant()` ptr-read assumes it; pin it.
#[test]
fn op_discriminant_in_range_for_every_op() {
    for op in all_ops() {
        let code = op.discriminant();
        assert!(
            (code as usize) < OpCode::COUNT,
            "discriminant {code} out of range for {op:?}"
        );
        // first byte of the encoding is the discriminant
        assert_eq!(
            encode_bytes(&op)[0],
            code,
            "first byte != discriminant for {op:?}"
        );
    }
}

// every Vec-carrying op must survive an EMPTY vec (the length-prefix == 0 path), which the
// non-empty samples above don't cover.
#[test]
fn empty_vec_fields_roundtrip() {
    roundtrip(Op::NewInstance {
        dst: r(1),
        adt: AdtId::from(11),
        fields: vec![],
    });
    roundtrip(Op::NewClosure {
        dst: r(1),
        body: BodyId::from(9),
        captures: vec![],
    });
    roundtrip(Op::Call {
        dst: r(1),
        callee: r(2),
        args: vec![],
    });
    roundtrip(Op::CallDirect {
        dst: r(1),
        body: BodyId::from(9),
        args: vec![],
    });
    roundtrip(Op::CallNative {
        dst: r(1),
        id: NativeId::from(13),
        args: vec![],
    });
    roundtrip(Op::Switch {
        scrut: r(1),
        base: 5,
        default: BlockTarget::ByteOffset(123),
        table: vec![],
    });
    roundtrip(Op::Format {
        dst: r(1),
        parts: vec![],
    });
}

// each Constant kind survives the round-trip (the Sample only covers Int); nested arrays too.
#[test]
fn all_constant_kinds_roundtrip() {
    for c in [
        Constant::Null,
        Constant::Bool(true),
        Constant::Bool(false),
        Constant::Int(-9223372036854775808),
        Constant::Float(3.5),
        Constant::Str(StrId::from(7)),
        Constant::Array(vec![]),
        Constant::Array(vec![
            Constant::Int(1),
            Constant::Bool(true),
            Constant::Array(vec![Constant::Str(StrId::from(7)), Constant::Null]),
        ]),
    ] {
        roundtrip(Op::LoadConst {
            dst: r(1),
            constant: c,
        });
    }
}

// edge int immediates and constants survive the i64 width exactly.
#[test]
fn extreme_int_immediate_roundtrips() {
    roundtrip(Op::AddIntImm {
        dst: r(0),
        left: r(1),
        val: i64::MAX,
    });
    roundtrip(Op::SubIntImm {
        dst: r(0),
        left: r(1),
        val: i64::MIN,
    });
    roundtrip(Op::LoadConst {
        dst: r(0),
        constant: Constant::Int(i64::MIN),
    });
    roundtrip(Op::LoadConst {
        dst: r(0),
        constant: Constant::Int(i64::MAX),
    });
}

#[test]
fn special_floats_roundtrip_bitexact() {
    for f in [
        0.0f64,
        -0.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN,
        f64::MAX,
        std::f64::consts::PI,
    ] {
        roundtrip(Op::LoadConst {
            dst: r(0),
            constant: Constant::Float(f),
        });
    }
    // NaN: can't roundtrip via Op compare, but the f64 codec must preserve its bits.
    let bytes = encode_bytes(&Op::LoadConst {
        dst: r(0),
        constant: Constant::Float(f64::NAN),
    });
    let mut d = Decoder { bytes, ip: 0 };
    let Op::LoadConst {
        constant: Constant::Float(got),
        ..
    } = decode_op(&mut d)
    else {
        panic!("expected float const")
    };
    assert!(got.is_nan(), "NaN const lost its NaN-ness");
}

// a high register index (> 255, so it can't fit a byte) must survive the 2-byte reg codec.
#[test]
fn wide_register_index_roundtrips() {
    roundtrip(Op::Move {
        dst: r(60000),
        src: r(1),
    });
}

// large switch table and many call args exercise the u16/u8 length prefixes.
#[test]
fn large_switch_and_arg_lists_roundtrip() {
    roundtrip(Op::Switch {
        scrut: r(0),
        base: 0,
        default: BlockTarget::ByteOffset(0),
        table: (0..300).map(BlockTarget::ByteOffset).collect(),
    });
    roundtrip(Op::Call {
        dst: r(0),
        callee: r(1),
        args: (0..200).map(r).collect(),
    });
}

// ir lowering: each source construct reaches the post-optimization op stream as the expected op

test_lowering!(
    int_literal_lowers_to_loadconst_int,
    "let x = 123456789;",
    Op::LoadConst {
        constant: Constant::Int(123456789),
        ..
    }
);
test_lowering!(
    float_literal_lowers_to_loadconst_float,
    "let x = 2.5;",
    Op::LoadConst {
        constant: Constant::Float(_),
        ..
    }
);
test_lowering!(
    str_literal_lowers_to_loadconst_str,
    "let x = \"hi\";",
    Op::LoadConst {
        constant: Constant::Str(_),
        ..
    }
);
test_lowering!(
    bool_literal_lowers_to_loadconst_bool,
    "let x = true;",
    Op::LoadConst {
        constant: Constant::Bool(true),
        ..
    }
);
test_lowering!(
    null_option_lowers_to_loadconst_null,
    "let x: int? = null;",
    Op::LoadConst {
        constant: Constant::Null,
        ..
    }
);
test_lowering!(
    enum_match_lowers_to_switch,
    "enum Color { Red, Green, Blue }
     fn f(c: Color) -> int { match c { Color::Red => 0, Color::Green => 1, Color::Blue => 2 } }
     let _ = f(Color::Red);",
    Op::Switch { .. }
);
test_lowering!(
    direct_index_uses_access_kind_direct,
    "let a = [1, 2, 3];\nlet x = a[0];",
    Op::GetIndex {
        kind: AccessKind::Direct,
        ..
    }
);
test_lowering!(
    dict_index_yields_get_index,
    "let d: ~{int} = ~{ k = 1 };\nlet x: int? = d[\"k\"];",
    Op::GetIndex { .. }
);
test_lowering!(
    int_compare_lowers_to_specialized_int_op,
    "let a = 1;\nlet b = 2;\nlet c = a < b;",
    Op::IntLt { .. } | Op::BIntLt { .. }
);
test_lowering!(
    float_arith_lowers_to_specialized_float_op,
    "let a = 1.0;\nlet b = 2.0;\nlet c = a + b;",
    Op::AddFloat { .. }
);
test_lowering!(
    str_eq_lowers_to_str_specialized_op,
    "let a = \"x\";\nlet b = \"y\";\nlet c = a == b;",
    Op::StrEq { .. }
);
// runtime params so the add isn't const-folded away
test_lowering!(
    int_add_lowers_to_int_specialized_op,
    "fn f(a: int, b: int) -> int { a + b }\nlet _ = f(1, 2);",
    Op::AddInt { .. } | Op::AddIntImm { .. }
);
test_lowering!(
    unary_negate_on_runtime_value_lowers_to_unary_or_neg,
    "fn f(a: int) -> int { -a }\nlet _ = f(1);",
    Op::Unary {
        op: UnaryOp::Negative,
        ..
    } | Op::SubInt { .. }
        | Op::MultIntImm { .. }
);
test_lowering!(
    logical_not_lowers_to_unary_not,
    "fn f(a: bool) -> bool { !a }\nlet _ = f(true);",
    Op::Unary {
        op: UnaryOp::Not,
        ..
    }
);
test_lowering!(
    return_lowers_to_return_op,
    "fn f(a: int) -> int { return a; }\nlet _ = f(1);",
    Op::Return { .. }
);
test_lowering!(
    struct_construction_lowers_to_new_instance,
    "struct P { x: int, y: int }
     fn make() -> P { P { x = 1, y = 2 } }
     let _ = make();",
    Op::NewInstance { .. }
);
test_lowering!(
    field_access_lowers_to_get_field,
    "struct P { x: int }
     fn get(p: P) -> int { p.x }
     let _ = get(P { x = 1 });",
    Op::GetField { .. }
);
// `for x in <array>` derives its bound from the array length -> Op::Len (a `.len()` call can't be
// tested here: the bare test Solver has no std library, so the method wouldn't resolve)
test_lowering!(
    for_in_array_lowers_bound_to_len_op,
    "let xs = [1, 2, 3];\nlet s = 0;\nfor x in xs { s = s + x; }",
    Op::Len { .. }
);
test_lowering!(
    for_in_collection_lowers_to_for_next,
    "let s = 0;\nfor i in 10 { s = s + i; }",
    Op::ForNext { .. }
);
test_lowering!(
    bool_literal_pattern_emits_bool_specialized_compare,
    "fn f(b: bool) -> int { match b { true => 1, false => 0 } }\nlet _ = f(true);",
    Op::BoolEq { .. } | Op::BoolNe { .. }
);

// shapes the single-pattern macro can't express (counts, absence, nested-field checks):

#[test]
fn array_literal_lowers_to_newarray_and_push() {
    let ops = compile_ops("let x = [1, 2, 3];");
    assert!(
        ops.iter().any(|o| matches!(o, Op::NewArray { .. })),
        "expected NewArray:\n{ops:#?}"
    );
    assert!(
        ops.iter().filter(|o| matches!(o, Op::Push { .. })).count() >= 3,
        "expected >=3 pushes:\n{ops:#?}"
    );
}

#[test]
fn dict_literal_lowers_to_newdict_and_insert() {
    let ops = compile_ops("let x: ~{int} = ~{ a = 1, b = 2 };");
    assert!(
        ops.iter().any(|o| matches!(o, Op::NewDict { .. })),
        "expected NewDict:\n{ops:#?}"
    );
    assert!(
        ops.iter()
            .filter(|o| matches!(o, Op::Insert { .. }))
            .count()
            >= 2,
        "expected >=2 inserts:\n{ops:#?}"
    );
}

#[test]
fn fstring_lowers_to_format_with_literal_and_value_parts() {
    let ops = compile_ops("let n = 3;\nlet s = f\"n is {n}!\";");
    let fmt = ops
        .iter()
        .find_map(|o| match o {
            Op::Format { parts, .. } => Some(parts),
            _ => None,
        })
        .expect("expected a Format op");
    assert!(
        fmt.iter().any(|p| matches!(p, OpFormatPart::Literal(_))),
        "expected a literal part:\n{fmt:#?}"
    );
    assert!(
        fmt.iter().any(|p| matches!(p, OpFormatPart::Value(_))),
        "expected a value part:\n{fmt:#?}"
    );
}

// direct `a == b` on bools must specialize to BoolEq/BoolNe, not fall back to a generic
// Bin{Identity}
#[test]
fn direct_bool_eq_lowers_to_specialized_bool_op() {
    let ops = compile_ops("fn f(a: bool, b: bool) -> bool { a == b }\nlet _ = f(true, false);");
    assert!(
        ops.iter().any(|o| matches!(o, Op::BoolEq { .. })),
        "bool == should lower to BoolEq:\n{ops:#?}"
    );
    assert!(
        !ops.iter().any(|o| matches!(
            o,
            Op::Bin {
                op: BinOp::Identity,
                ..
            }
        )),
        "bool == should not fall back to a generic Bin:\n{ops:#?}"
    );

    let ops = compile_ops("fn f(a: bool, b: bool) -> bool { a != b }\nlet _ = f(true, false);");
    assert!(
        ops.iter().any(|o| matches!(o, Op::BoolNe { .. })),
        "bool != should lower to BoolNe:\n{ops:#?}"
    );
}
