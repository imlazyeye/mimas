use std::{any::TypeId, slice::Iter};

use bevy::{
    math::{Quat, Vec2, Vec3},
    prelude::Entity,
    reflect::{NamedField, PartialReflect, TypeInfo, UnnamedField, enums::VariantInfo},
};

// my brain is truly broken after the vm, this is the only way I can think now
macro_rules! with_ints {
    ($then:ident) => {
        $then!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize)
    };
}

/// How a reflected type gets converted for scripts.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Bool,
    Int(TypeId),
    F32,
    F64,
    Str,
    Vec2,
    Vec3,
    Quat,
    Entity,
    Option(&'static TypeInfo),
    List(&'static TypeInfo),
    Type(TypeId),
}

/// Figures out the [`Kind`] of a type, or `None` if scripts can't use it (like a map).
pub(crate) fn kind(info: &'static TypeInfo) -> Option<Kind> {
    let id = info.type_id();
    macro_rules! is_int {
        ($($t:ty),*) => {
            $(id == TypeId::of::<$t>())||*
        };
    }

    // one of those matches where you ask "am i stupid or am i god's greatest programmer"
    Some(match info {
        _ if id == TypeId::of::<bool>() => Kind::Bool,
        _ if with_ints!(is_int) => Kind::Int(id),
        _ if id == TypeId::of::<f32>() => Kind::F32,
        _ if id == TypeId::of::<f64>() => Kind::F64,
        _ if id == TypeId::of::<String>() => Kind::Str,
        _ if id == TypeId::of::<Vec2>() => Kind::Vec2,
        _ if id == TypeId::of::<Vec3>() => Kind::Vec3,
        _ if id == TypeId::of::<Quat>() => Kind::Quat,
        _ if id == TypeId::of::<Entity>() => Kind::Entity,
        TypeInfo::Enum(option) if info.type_path().starts_with("core::option::Option<") => {
            let Some(VariantInfo::Tuple(some)) = option.variant("Some") else {
                return None;
            };
            Kind::Option(some.field_at(0)?.type_info()?)
        }
        TypeInfo::List(list) => Kind::List(list.item_info()?),
        TypeInfo::Struct(_) | TypeInfo::TupleStruct(_) | TypeInfo::Enum(_) => Kind::Type(id),
        _ => return None,
    })
}

/// Reads any integer as an `int`. A `u64` above `i64::MAX` keeps its bits and comes out negative.
pub(crate) fn read_int(value: &dyn PartialReflect) -> Option<i64> {
    macro_rules! read {
        ($($t:ty),*) => {
            $(if let Some(n) = value.try_downcast_ref::<$t>() {
                return Some(*n as i64);
            })*
        };
    }
    with_ints!(read);
    None
}

/// Boxes an `int` as the integer type `id`. 64-bit integers take the bits back as-is, and
/// smaller ones have to fit.
pub(crate) fn box_int(id: TypeId, n: i64) -> Option<Box<dyn PartialReflect>> {
    macro_rules! boxed {
        ($($t:ty),*) => {
            $(if id == TypeId::of::<$t>() {
                let n = if size_of::<$t>() == size_of::<i64>() {
                    n as $t
                } else {
                    <$t>::try_from(n).ok()?
                };
                return Some(Box::new(n));
            })*
        };
    }
    with_ints!(boxed);
    None
}

/// The name scripts use for the type (without its module).
pub(crate) fn name(info: &TypeInfo) -> &'static str {
    info.type_path_table().short_path()
}

/// Iterates the fields of a struct or enum variant, in order.
pub(crate) enum Members {
    Unit,
    Named(Iter<'static, NamedField>),
    Tuple(Iter<'static, UnnamedField>),
}

impl Members {
    /// The fields, assuming they're all reflected (true for anything in the catalog).
    pub fn reflected(self) -> impl Iterator<Item = (Option<&'static str>, &'static TypeInfo)> {
        self.map(|(name, info)| {
            (
                name,
                info.expect("the catalog only takes types whose fields are all reflected"),
            )
        })
    }

    /// The fields of a struct, or of the `variant`th variant of an enum.
    pub fn of(info: &'static TypeInfo, variant: usize) -> Option<Self> {
        Some(match info {
            TypeInfo::Struct(fields) => Members::Named(fields.iter()),
            TypeInfo::TupleStruct(fields) => Members::Tuple(fields.iter()),
            TypeInfo::Enum(variants) => match variants.variant_at(variant)? {
                VariantInfo::Unit(_) => Members::Unit,
                VariantInfo::Tuple(fields) => Members::Tuple(fields.iter()),
                VariantInfo::Struct(fields) => Members::Named(fields.iter()),
            },
            _ => return None,
        })
    }
}

impl Iterator for Members {
    /// A field's name (if it has one) and type info (if it's reflected).
    type Item = (Option<&'static str>, Option<&'static TypeInfo>);

    fn next(&mut self) -> Option<Self::Item> {
        Some(match self {
            Members::Unit => return None,
            Members::Named(fields) => {
                let field = fields.next()?;
                (Some(field.name()), field.type_info())
            }
            Members::Tuple(fields) => (None, fields.next()?.type_info()),
        })
    }
}
