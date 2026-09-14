use std::collections::HashMap;

use api::NativeId;
use parse::AccessKind;
use shared::{IdVec, Location, StrId, StrInterner};
use solve::components::AdtId;

use crate::{
    BinOp, BlockTarget, BodyId, Constant, ConstantCode, OpCode, OpFormatPart, OpFormatPartCode,
    Reg, UnaryOp,
};

pub struct Program {
    pub entry: BodyId,
    pub chunks: IdVec<BodyId, Chunk>,
    pub strs: StrInterner,
    pub bytes: Vec<u8>,
    pub items: HashMap<String, BodyId>,
    /// Struct/variant names, indexed by the same `AdtId`/`struct_id` that `NewInstance` and
    /// runtime instances carry -- lets `print`/`display` show `Node { .. }` instead of `@3 { .. }`.
    pub struct_names: IdVec<AdtId, String>,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub offset: usize,
    pub args: u16,
    pub regs: u16,
    pub params: Vec<Reg>,
    pub captures: Vec<Reg>,
    pub locals: HashMap<String, Reg>,
    // chunk-relative byte offset -> loc that applies from that offset up to the next entry.
    // sorted and deduped against the previous loc -- hermes/v8-style sparse table. consult via
    // `loc_at`; an empty table or an ip before the first entry resolves to `Location::SYNTHETIC`.
    pub locs: Vec<(u32, Location)>,
}

impl Chunk {
    /// Resolve the loc for the opcode whose start byte is at chunk-relative `ip`.
    pub fn loc_at(&self, ip: u32) -> Location {
        let idx = self.locs.partition_point(|(off, _)| *off <= ip);
        if idx == 0 {
            Location::SYNTHETIC
        } else {
            self.locs[idx - 1].1
        }
    }
}

pub(crate) struct Encoder(Vec<u8>);
impl Encoder {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn finish(self) -> Vec<u8> {
        self.0
    }

    pub fn u8(&mut self, v: impl Into<u8>) {
        self.0.push(v.into());
    }

    pub fn reg(&mut self, v: Reg) {
        self.u16(u16::try_from(u32::from(v)).unwrap());
    }

    pub fn u16(&mut self, v: impl Into<u16>) {
        self.0.extend(v.into().to_le_bytes());
    }

    pub fn u32(&mut self, v: impl Into<u32>) {
        self.0.extend(v.into().to_le_bytes());
    }

    pub fn i64(&mut self, v: impl Into<i64>) {
        self.0.extend(v.into().to_le_bytes());
    }

    pub fn f64(&mut self, v: impl Into<f64>) {
        self.0.extend(v.into().to_le_bytes());
    }
}

pub struct Decoder {
    pub bytes: Vec<u8>,
    pub ip: usize,
}

impl Decoder {
    #[inline(always)]
    pub fn u8(&mut self) -> u8 {
        debug_assert!(
            self.ip < self.bytes.len(),
            "decode past end: ip {} len {}",
            self.ip,
            self.bytes.len()
        );
        let v = unsafe { *self.bytes.as_ptr().add(self.ip) };
        self.ip += 1;
        v
    }

    #[inline(always)]
    pub fn u16(&mut self) -> u16 {
        debug_assert!(
            self.ip + 2 <= self.bytes.len(),
            "decode past end: ip {} len {}",
            self.ip,
            self.bytes.len()
        );
        let v = unsafe {
            self.bytes
                .as_ptr()
                .add(self.ip)
                .cast::<u16>()
                .read_unaligned()
        };
        self.ip += 2;
        u16::from_le(v)
    }

    #[inline(always)]
    pub fn u32(&mut self) -> u32 {
        debug_assert!(
            self.ip + 4 <= self.bytes.len(),
            "decode past end: ip {} len {}",
            self.ip,
            self.bytes.len()
        );
        let v = unsafe {
            self.bytes
                .as_ptr()
                .add(self.ip)
                .cast::<u32>()
                .read_unaligned()
        };
        self.ip += 4;
        u32::from_le(v)
    }

    #[inline(always)]
    pub fn peek_u32(&self, at: usize) -> u32 {
        debug_assert!(
            at + 4 <= self.bytes.len(),
            "peek past end: at {} len {}",
            at,
            self.bytes.len()
        );
        let v = unsafe { self.bytes.as_ptr().add(at).cast::<u32>().read_unaligned() };
        u32::from_le(v)
    }

    #[inline(always)]
    pub fn i64(&mut self) -> i64 {
        debug_assert!(
            self.ip + 8 <= self.bytes.len(),
            "decode past end: ip {} len {}",
            self.ip,
            self.bytes.len()
        );
        let v = unsafe {
            self.bytes
                .as_ptr()
                .add(self.ip)
                .cast::<i64>()
                .read_unaligned()
        };
        self.ip += 8;
        i64::from_le(v)
    }

    #[inline(always)]
    pub fn f64(&mut self) -> f64 {
        debug_assert!(
            self.ip + 8 <= self.bytes.len(),
            "decode past end: ip {} len {}",
            self.ip,
            self.bytes.len()
        );
        let v = unsafe {
            self.bytes
                .as_ptr()
                .add(self.ip)
                .cast::<u64>()
                .read_unaligned()
        };
        self.ip += 8;
        f64::from_bits(u64::from_le(v))
    }
}

pub trait Decode: Sized {
    fn decode(decoder: &mut Decoder) -> Self;
}

impl Decode for bool {
    #[inline(always)]
    fn decode(decoder: &mut Decoder) -> Self {
        decoder.u8() != 0
    }
}

impl Decode for OpCode {
    #[inline(always)]
    fn decode(decoder: &mut Decoder) -> Self {
        let b = decoder.u8();
        debug_assert!(
            (b as usize) < OpCode::COUNT,
            "invalid OpCode byte {b}: bytecode IP desync (codegen bug)"
        );
        unsafe { std::mem::transmute::<u8, OpCode>(b) }
    }
}

impl Decode for BinOp {
    #[inline(always)]
    fn decode(decoder: &mut Decoder) -> Self {
        let b = decoder.u8();
        debug_assert!(
            (b as usize) <= BinOp::Coalesce as usize,
            "invalid BinOp byte {b}: bytecode desync"
        );
        unsafe { std::mem::transmute::<u8, BinOp>(b) }
    }
}

impl Decode for UnaryOp {
    #[inline(always)]
    fn decode(decoder: &mut Decoder) -> Self {
        let b = decoder.u8();
        debug_assert!(
            (b as usize) <= UnaryOp::Negative as usize,
            "invalid UnaryOp byte {b}: bytecode desync"
        );
        unsafe { std::mem::transmute::<u8, UnaryOp>(b) }
    }
}

impl Decode for Constant {
    #[inline(always)]
    fn decode(decoder: &mut Decoder) -> Self {
        let b = decoder.u8();
        debug_assert!(
            (b as usize) <= ConstantCode::Null as usize,
            "invalid ConstantCode byte {b}: bytecode desync"
        );
        let code = unsafe { std::mem::transmute::<u8, ConstantCode>(b) };
        match code {
            ConstantCode::Bool => Constant::Bool(decoder.u8() == 1),
            ConstantCode::Int => Constant::Int(decoder.i64()),
            ConstantCode::Float => Constant::Float(decoder.f64()),
            ConstantCode::Str => Constant::Str(StrId::from(decoder.u32())),
            ConstantCode::Array => {
                let len = decoder.u32() as usize;
                let mut items = Vec::with_capacity(len);
                for _ in 0..len {
                    items.push(Constant::decode(decoder));
                }
                Constant::Array(items)
            }
            ConstantCode::Null => Constant::Null,
        }
    }
}

impl Decode for Reg {
    #[inline(always)]
    fn decode(decoder: &mut Decoder) -> Self {
        Reg::from(u32::from(decoder.u16()))
    }
}

impl Decode for BodyId {
    #[inline(always)]
    fn decode(decoder: &mut Decoder) -> Self {
        BodyId::from(decoder.u32())
    }
}

impl Decode for AdtId {
    #[inline(always)]
    fn decode(decoder: &mut Decoder) -> Self {
        AdtId::from(decoder.u32())
    }
}

impl Decode for NativeId {
    #[inline(always)]
    fn decode(decoder: &mut Decoder) -> Self {
        NativeId::from(decoder.u32())
    }
}

impl Decode for BlockTarget {
    #[inline(always)]
    fn decode(decoder: &mut Decoder) -> Self {
        BlockTarget::ByteOffset(decoder.u32() as usize)
    }
}

impl Decode for AccessKind {
    #[inline(always)]
    fn decode(decoder: &mut Decoder) -> Self {
        let b = decoder.u8();
        debug_assert!(
            (b as usize) <= AccessKind::Option as usize,
            "invalid AccessKind byte {b}: bytecode desync"
        );
        unsafe { std::mem::transmute::<u8, AccessKind>(b) }
    }
}

impl Decode for OpFormatPart {
    #[inline(always)]
    fn decode(decoder: &mut Decoder) -> Self {
        let b = decoder.u8();
        debug_assert!(
            (b as usize) <= OpFormatPartCode::Value as usize,
            "invalid OpFormatPartCode byte {b}: bytecode desync"
        );
        let code = unsafe { std::mem::transmute::<u8, OpFormatPartCode>(b) };
        match code {
            OpFormatPartCode::Literal => OpFormatPart::Literal(StrId::from(decoder.u32())),
            OpFormatPartCode::Value => OpFormatPart::Value(Reg::decode(decoder)),
        }
    }
}
