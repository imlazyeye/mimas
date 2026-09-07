use colored::Colorize;
use itertools::Itertools;
use parse::AccessKind;
use shared::StrId;
use solve::components::AdtId;

use crate::{BlockTarget, BodyId, Compiler, Constant, Op, OpFormatPart, Reg};

const OP_WIDTH: usize = 18;
const ARG_WIDTH: usize = 10;

#[mutants::skip]
impl std::fmt::Display for Compiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.ops
                .iter()
                .enumerate()
                .map(|(offset, op)| format!("{}    {op}", Offset(offset)))
                .join("\n")
        )
    }
}

macro_rules! cmp {
    ($f:ident, $name:literal, $target:expr, $left:expr, $rhs:expr) => {
        write!($f, "{}{}{}{}", OpName($name), $target, $left, $rhs,)
    };
}

// render a compare-and-branch op. $name picks the _true/_false suffix; $rhs is the second
// operand -- a register for the reg-reg form, or a ConstantFmt for the immediate form.
macro_rules! bcmp {
    ($f:ident, $name:literal, $t:expr, $target:expr, $left:expr, $rhs:expr) => {
        write!(
            $f,
            "{}{}{}{}",
            if *$t {
                OpName(concat!($name, "_true"))
            } else {
                OpName(concat!($name, "_false"))
            },
            $target,
            $left,
            $rhs,
        )
    };
}

/// this is pretty dumb and should be in the macro but i dont care right now
#[mutants::skip]
impl std::fmt::Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::LoadConst { dst, constant } => write!(
                f,
                "{}{}{}",
                OpName("load_const"),
                dst,
                ConstantFmt(constant)
            ),
            Op::Move { dst, src } => write!(f, "{}{}{}", OpName("move"), dst, src),

            Op::Bin {
                dst,
                left,
                op,
                right,
            } => {
                write!(
                    f,
                    "{}{}{}{}",
                    OpKind("bin", &format!("{op:?}").to_lowercase()),
                    dst,
                    left,
                    right
                )
            }

            Op::Unary { dst, op, src } => {
                write!(
                    f,
                    "{}{}{}",
                    OpKind("unary", &format!("{op:?}").to_lowercase()),
                    dst,
                    src
                )
            }

            Op::Jump { target } => write!(f, "{}{}", OpName("jump"), target),

            Op::ForNext { idx, bound, target } => {
                write!(f, "{}{}{}{}", OpName("for_next"), idx, bound, target)
            }

            Op::JumpIf {
                cond,
                target,
                is_true,
            } => {
                write!(
                    f,
                    "{}{}{}",
                    if *is_true {
                        OpName("jump_if_true")
                    } else {
                        OpName("jump_if_false")
                    },
                    cond,
                    target
                )
            }

            Op::Switch {
                scrut,
                base,
                default,
                table,
            } => {
                write!(f, "{}{}", OpName("switch"), scrut)?;
                for (i, t) in table.iter().enumerate() {
                    write!(f, " {}={t}", *base + i as u32)?;
                }
                write!(f, " else{default}")
            }

            Op::NewArray { dst } => write!(f, "{}{}", OpName("new_array"), dst),
            Op::NewDict { dst } => write!(f, "{}{}", OpName("new_dict"), dst),

            Op::GetIndex {
                dst,
                set,
                index,
                kind,
            } => {
                write!(
                    f,
                    "{}{}{}{}{}",
                    OpName("get_index"),
                    dst,
                    set,
                    index,
                    if kind == &AccessKind::Direct { "" } else { "?" },
                )
            }

            Op::SetIndex { set, index, value } => {
                write!(f, "{}{}{}{}", OpName("set_index"), set, index, value)
            }

            Op::Push { array, value } => write!(f, "{}{}{}", OpName("push"), array, value),

            Op::Insert { dict, key, value } => {
                write!(f, "{}{}{}{}", OpName("insert"), dict, Field(*key), value)
            }

            Op::Len { dst, src: value } => write!(f, "{}{}{}", OpName("len"), dst, value),
            Op::ToFloat { dst, src: value } => write!(f, "{}{}{}", OpName("to_float"), dst, value),
            Op::Sqrt { dst, src: value } => write!(f, "{}{}{}", OpName("sqrt"), dst, value),
            Op::Unwrap { dst, src: value } => write!(f, "{}{}{}", OpName("unwrap"), dst, value),

            Op::In {
                dst,
                needle,
                haystack,
                condition,
            } => {
                write!(
                    f,
                    "{}{}{}{}",
                    if *condition {
                        OpName("in")
                    } else {
                        OpName("!in")
                    },
                    dst,
                    needle,
                    haystack
                )
            }

            Op::Format { dst, parts } => {
                write!(f, "{}{}", OpName("format"), dst)?;

                for part in parts {
                    match part {
                        OpFormatPart::Literal(lit) => write!(f, "{}", Field(*lit))?,
                        OpFormatPart::Value(value) => write!(f, "{value}")?,
                    }
                }

                Ok(())
            }

            Op::LoadBody { dst, body } => {
                write!(f, "{}{}{}", OpName("load_body"), dst, Body(*body))
            }

            Op::Call { dst, callee, args } => {
                write!(f, "{}{}{}", OpName("call"), dst, callee)?;
                for arg in args {
                    write!(f, "{arg}")?;
                }
                Ok(())
            }

            Op::Return { val: value } => write!(f, "{}{}", OpName("return"), value),

            Op::NewInstance { dst, adt, fields } => {
                write!(f, "{}{}{}", OpName("instance"), dst, Adt(*adt))?;
                for field in fields {
                    write!(f, "{field}")?;
                }
                Ok(())
            }

            Op::CallDirect { dst, body, args } => {
                write!(f, "{}{}{}", OpName("call_direct"), dst, Body(*body))?;
                for arg in args {
                    write!(f, "{arg}")?;
                }
                Ok(())
            }

            Op::GetField {
                dst,
                src,
                slot,
                kind,
            } => {
                write!(
                    f,
                    "{}{}{}.{slot}{}",
                    OpName("get_field"),
                    dst,
                    src,
                    if kind == &AccessKind::Direct { "" } else { "?" },
                )
            }

            Op::SetField {
                receiver,
                slot,
                value,
            } => write!(f, "{}{}.{slot}{}", OpName("set_field"), receiver, value),

            Op::CallNative { dst, id, args } => {
                write!(f, "{}{}#{}", OpName("call_native"), dst, id.index())?;
                for arg in args {
                    write!(f, "{arg}")?;
                }
                Ok(())
            }

            Op::Panic {} => write!(f, "panic"),
            Op::IsInstance { dst, src, adt } => {
                write!(f, "is_instance {dst} = {src} @{}", adt.index())
            }
            Op::NewClosure {
                dst,
                body,
                captures,
            } => {
                write!(f, "{}{}{}", OpName("new_closure"), dst, Body(*body))?;
                for c in captures {
                    write!(f, "{c}")?;
                }
                Ok(())
            }
            Op::Raise { val: src } => write!(f, "{}{}", OpName("raise"), src),
            Op::IsRaised { dst, src } => write!(f, "{}{}{}", OpName("is_raised"), dst, src),
            Op::UnwrapRaised { dst, src } => {
                write!(f, "{}{}{}", OpName("unwrap_raised"), dst, src)
            }
            Op::BoolEq { dst, left, right } => cmp!(f, "bool_eq", dst, left, right),
            Op::BoolNe { dst, left, right } => cmp!(f, "bool_ne", dst, left, right),
            Op::AddInt { dst, left, right } => cmp!(f, "add_int", dst, left, right),
            Op::ModInt { dst, left, right } => cmp!(f, "mod_int", dst, left, right),
            Op::SubInt { dst, left, right } => cmp!(f, "sub_int", dst, left, right),
            Op::MultInt { dst, left, right } => cmp!(f, "mult_int", dst, left, right),
            Op::AddFloat { dst, left, right } => cmp!(f, "add_float", dst, left, right),
            Op::SubFloat { dst, left, right } => cmp!(f, "sub_float", dst, left, right),
            Op::MultFloat { dst, left, right } => cmp!(f, "mult_float", dst, left, right),
            Op::DivFloat { dst, left, right } => cmp!(f, "div_float", dst, left, right),
            Op::IntLt { dst, left, right } => cmp!(f, "int_lt", dst, left, right),
            Op::IntLe { dst, left, right } => cmp!(f, "int_le", dst, left, right),
            Op::IntGt { dst, left, right } => cmp!(f, "int_gt", dst, left, right),
            Op::IntGe { dst, left, right } => cmp!(f, "int_ge", dst, left, right),
            Op::IntEq { dst, left, right } => cmp!(f, "int_eq", dst, left, right),
            Op::IntNe { dst, left, right } => cmp!(f, "int_ne", dst, left, right),
            Op::FloatLt { dst, left, right } => cmp!(f, "float_lt", dst, left, right),
            Op::FloatLe { dst, left, right } => cmp!(f, "float_le", dst, left, right),
            Op::FloatGt { dst, left, right } => cmp!(f, "float_gt", dst, left, right),
            Op::FloatGe { dst, left, right } => cmp!(f, "float_ge", dst, left, right),
            Op::FloatEq { dst, left, right } => cmp!(f, "float_eq", dst, left, right),
            Op::FloatNe { dst, left, right } => cmp!(f, "float_ne", dst, left, right),
            Op::StrEq { dst, left, right } => cmp!(f, "str_eq", dst, left, right),
            Op::StrNe { dst, left, right } => cmp!(f, "str_ne", dst, left, right),
            Op::BIntLt {
                target,
                left,
                right,
                is_true,
            } => bcmp!(f, "bint_lt", is_true, target, left, right),
            Op::BIntLe {
                target,
                left,
                right,
                is_true,
            } => bcmp!(f, "bint_le", is_true, target, left, right),
            Op::BIntGt {
                target,
                left,
                right,
                is_true,
            } => bcmp!(f, "bint_gt", is_true, target, left, right),
            Op::BIntGe {
                target,
                left,
                right,
                is_true,
            } => bcmp!(f, "bint_ge", is_true, target, left, right),
            Op::BIntEq {
                target,
                left,
                right,
                is_true,
            } => bcmp!(f, "bint_eq", is_true, target, left, right),
            Op::BIntNe {
                target,
                left,
                right,
                is_true,
            } => bcmp!(f, "bint_ne", is_true, target, left, right),
            Op::AddIntImm { dst, left, val } => cmp!(
                f,
                "add_int_imm",
                dst,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::SubIntImm { dst, left, val } => cmp!(
                f,
                "sub_int_imm",
                dst,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::MultIntImm { dst, left, val } => cmp!(
                f,
                "mult_int_imm",
                dst,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::ModIntImm { dst, left, val } => cmp!(
                f,
                "mod_int_imm",
                dst,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::IntLtImm { dst, left, val } => cmp!(
                f,
                "int_lt_imm",
                dst,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::IntLeImm { dst, left, val } => cmp!(
                f,
                "int_le_imm",
                dst,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::IntGtImm { dst, left, val } => cmp!(
                f,
                "int_gt_imm",
                dst,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::IntGeImm { dst, left, val } => cmp!(
                f,
                "int_ge_imm",
                dst,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::IntEqImm { dst, left, val } => cmp!(
                f,
                "int_eq_imm",
                dst,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::IntNeImm { dst, left, val } => cmp!(
                f,
                "int_ne_imm",
                dst,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::AddFloatImm { dst, left, val } => cmp!(
                f,
                "add_float_imm",
                dst,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::SubFloatImm { dst, left, val } => cmp!(
                f,
                "sub_float_imm",
                dst,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::MultFloatImm { dst, left, val } => cmp!(
                f,
                "mult_float_imm",
                dst,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::ModFloatImm { dst, left, val } => cmp!(
                f,
                "mod_float_imm",
                dst,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::FloatLtImm { dst, left, val } => cmp!(
                f,
                "float_lt_imm",
                dst,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::FloatLeImm { dst, left, val } => cmp!(
                f,
                "float_le_imm",
                dst,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::FloatGtImm { dst, left, val } => cmp!(
                f,
                "float_gt_imm",
                dst,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::FloatGeImm { dst, left, val } => cmp!(
                f,
                "float_ge_imm",
                dst,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::FloatEqImm { dst, left, val } => cmp!(
                f,
                "float_eq_imm",
                dst,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::FloatNeImm { dst, left, val } => cmp!(
                f,
                "float_ne_imm",
                dst,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::BIntLtImm {
                target,
                left,
                val,
                is_true,
            } => bcmp!(
                f,
                "bint_lt_imm",
                is_true,
                target,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::BIntLeImm {
                target,
                left,
                val,
                is_true,
            } => bcmp!(
                f,
                "bint_le_imm",
                is_true,
                target,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::BIntGtImm {
                target,
                left,
                val,
                is_true,
            } => bcmp!(
                f,
                "bint_gt_imm",
                is_true,
                target,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::BIntGeImm {
                target,
                left,
                val,
                is_true,
            } => bcmp!(
                f,
                "bint_ge_imm",
                is_true,
                target,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::BIntEqImm {
                target,
                left,
                val,
                is_true,
            } => bcmp!(
                f,
                "bint_eq_imm",
                is_true,
                target,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::BIntNeImm {
                target,
                left,
                val,
                is_true,
            } => bcmp!(
                f,
                "bint_ne_imm",
                is_true,
                target,
                left,
                ConstantFmt(&Constant::Int(*val))
            ),
            Op::BFloatLtImm {
                target,
                left,
                val,
                is_true,
            } => bcmp!(
                f,
                "bfloat_lt_imm",
                is_true,
                target,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::BFloatLeImm {
                target,
                left,
                val,
                is_true,
            } => bcmp!(
                f,
                "bfloat_le_imm",
                is_true,
                target,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::BFloatGtImm {
                target,
                left,
                val,
                is_true,
            } => bcmp!(
                f,
                "bfloat_gt_imm",
                is_true,
                target,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::BFloatGeImm {
                target,
                left,
                val,
                is_true,
            } => bcmp!(
                f,
                "bfloat_ge_imm",
                is_true,
                target,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::BFloatEqImm {
                target,
                left,
                val,
                is_true,
            } => bcmp!(
                f,
                "bfloat_eq_imm",
                is_true,
                target,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::BFloatNeImm {
                target,
                left,
                val,
                is_true,
            } => bcmp!(
                f,
                "bfloat_ne_imm",
                is_true,
                target,
                left,
                ConstantFmt(&Constant::Float(f64::from_bits(*val as u64)))
            ),
            Op::BFloatLt {
                target,
                left,
                right,
                is_true,
            } => bcmp!(f, "bfloat_lt", is_true, target, left, right),
            Op::BFloatLe {
                target,
                left,
                right,
                is_true,
            } => bcmp!(f, "bfloat_le", is_true, target, left, right),
            Op::BFloatGt {
                target,
                left,
                right,
                is_true,
            } => bcmp!(f, "bfloat_gt", is_true, target, left, right),
            Op::BFloatGe {
                target,
                left,
                right,
                is_true,
            } => bcmp!(f, "bfloat_ge", is_true, target, left, right),
            Op::BFloatEq {
                target,
                left,
                right,
                is_true,
            } => bcmp!(f, "bfloat_eq", is_true, target, left, right),
            Op::BFloatNe {
                target,
                left,
                right,
                is_true,
            } => bcmp!(f, "bfloat_ne", is_true, target, left, right),
        }
    }
}

#[mutants::skip]
impl std::fmt::Display for Reg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = format!("r{}", self.index())
            .bright_cyan()
            .bold()
            .to_string();
        write_padded(f, text, 1 + self.index().to_string().len(), ARG_WIDTH)
    }
}

#[mutants::skip]
impl std::fmt::Display for BlockTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockTarget::Block(block) => {
                let text = format!("b{}", block.index()).bright_yellow().to_string();
                write_padded(f, text, 1 + block.index().to_string().len(), ARG_WIDTH)
            }
            BlockTarget::ByteOffset(offset) => {
                let text = format!("{offset:04}").bright_yellow().to_string();
                write_padded(f, text, 4, ARG_WIDTH)
            }
        }
    }
}

struct Offset(usize);
struct OpName(&'static str);
struct OpKind<'a>(&'static str, &'a str);
struct ConstantFmt<'a>(&'a Constant);
struct Field(StrId);
struct Body(BodyId);
struct Adt(AdtId);

#[mutants::skip]
impl std::fmt::Display for Offset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:04}", self.0).bright_black().bold())
    }
}

#[mutants::skip]
impl std::fmt::Display for OpName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_padded(
            f,
            self.0.bright_white().bold().to_string(),
            self.0.len(),
            OP_WIDTH,
        )
    }
}

#[mutants::skip]
impl std::fmt::Display for OpKind<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = format!(
            "{} {}{}{}",
            self.0.bright_white().bold(),
            "(".bright_black(),
            self.1.bright_cyan().bold(),
            ")".bright_black(),
        );

        write_padded(f, text, self.0.len() + self.1.len() + 3, OP_WIDTH)
    }
}

#[mutants::skip]
impl std::fmt::Display for ConstantFmt<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = format!("{}", self.0);
        write_padded(f, text.bright_green().to_string(), text.len(), ARG_WIDTH)
    }
}

#[mutants::skip]
impl std::fmt::Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        dotted(f, ".", self.0.index())
    }
}

#[mutants::skip]
impl std::fmt::Display for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        dotted(f, "@", self.0.index())
    }
}

#[mutants::skip]
impl std::fmt::Display for Adt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        dotted(f, "$", self.0.index())
    }
}

fn dotted(f: &mut std::fmt::Formatter<'_>, prefix: &str, index: usize) -> std::fmt::Result {
    let digits = index.to_string();

    let text = format!("{}{}", prefix.bright_black(), digits.purple(),);

    write_padded(f, text, prefix.len() + digits.len(), ARG_WIDTH)
}

fn write_padded(
    f: &mut std::fmt::Formatter<'_>,
    text: String,
    visible_width: usize,
    width: usize,
) -> std::fmt::Result {
    write!(
        f,
        "{}{}",
        text,
        " ".repeat(width.saturating_sub(visible_width))
    )
}
