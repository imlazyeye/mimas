use std::any::TypeId;

use bevy::{
    math::{Quat, Vec2, Vec3},
    prelude::Entity,
    reflect::{
        FromReflect, PartialReflect, ReflectRef, TypeInfo, Typed,
        enums::{DynamicEnum, DynamicVariant},
        list::DynamicList,
        structs::DynamicStruct,
        tuple::DynamicTuple,
        tuple_struct::DynamicTupleStruct,
    },
};

use super::kind::{Kind, Members, box_int, kind, read_int};
use crate::types::Entity as ScriptEntity;
use vm::{Ctx, Fields, Val, conversion::MimasType};

/// Moves values between Rust and scripts.
pub(crate) struct Convert;

impl Convert {
    /// Turns a reflected value into a script value.
    pub fn to_mimas<'gc>(
        ctx: Ctx<'gc>,
        info: &'static TypeInfo,
        value: &dyn PartialReflect,
    ) -> Option<Val<'gc>> {
        Some(match kind(info)? {
            Kind::Bool => Val::Bool(*value.try_downcast_ref::<bool>()?),
            Kind::Int(_) => Val::Int(read_int(value)?),
            Kind::F32 => Val::Float(*value.try_downcast_ref::<f32>()? as f64),
            Kind::F64 => Val::Float(*value.try_downcast_ref::<f64>()?),
            Kind::Str => Val::Str(ctx.intern(value.try_downcast_ref::<String>()?)),
            Kind::Vec2 => (*value.try_downcast_ref::<Vec2>()?).into_value(ctx),
            Kind::Vec3 => (*value.try_downcast_ref::<Vec3>()?).into_value(ctx),
            Kind::Quat => (*value.try_downcast_ref::<Quat>()?).into_value(ctx),
            Kind::Entity => {
                ScriptEntity::from(*value.try_downcast_ref::<Entity>()?).into_value(ctx)
            }
            Kind::Option(inner) => {
                let ReflectRef::Enum(option) = value.reflect_ref() else {
                    return None;
                };
                match option.variant_name() {
                    "Some" => Self::to_mimas(ctx, inner, option.field_at(0)?)?,
                    _ => Val::Null,
                }
            }
            Kind::List(inner) => {
                let ReflectRef::List(list) = value.reflect_ref() else {
                    return None;
                };
                let items = list
                    .iter()
                    .map(|item| Self::to_mimas(ctx, inner, item))
                    .collect::<Option<Vec<_>>>()?;
                Val::Array(ctx.new_array(items))
            }
            Kind::Type(id) => {
                let binding = ctx.binding(id)?;
                let value = value.reflect_ref();
                let (layout, members) = match &value {
                    ReflectRef::Enum(value) => {
                        let index = value.variant_index();
                        (
                            *binding.variant_layout_ids.get(index)?,
                            Members::of(info, index)?,
                        )
                    }
                    _ => (binding.adt_id, Members::of(info, 0)?),
                };
                let fields = members
                    .reflected()
                    .enumerate()
                    .map(|(i, (_, member))| Self::to_mimas(ctx, member, Self::field(&value, i)?))
                    .collect::<Option<Vec<_>>>()?;
                Val::Instance(ctx.new_instance(layout.index() as u32, Fields::new(fields)))
            }
        })
    }

    /// Gets a real `T` out of a script value.
    pub fn to_rust<'gc, T: FromReflect + Typed>(ctx: Ctx<'gc>, value: Val<'gc>) -> Option<T> {
        T::from_reflect(&*Self::to_reflect(ctx, T::type_info(), value)?)
    }

    /// Builds a dynamic reflected value of `info`'s type from a script value.
    pub fn to_reflect<'gc>(
        ctx: Ctx<'gc>,
        info: &'static TypeInfo,
        value: Val<'gc>,
    ) -> Option<Box<dyn PartialReflect>> {
        Some(match kind(info)? {
            Kind::Bool => Box::new(value.as_bool()?),
            Kind::Int(id) => box_int(id, value.as_int()?)?,
            Kind::F32 => Box::new(value.as_float()? as f32),
            Kind::F64 => Box::new(value.as_float()?),
            Kind::Str => Box::new(value.as_str()?.as_str().to_string()),
            Kind::Vec2 => Box::new(Vec2::from_value(ctx, value).ok()?),
            Kind::Vec3 => Box::new(Vec3::from_value(ctx, value).ok()?),
            Kind::Quat => Box::new(Quat::from_value(ctx, value).ok()?),
            Kind::Entity => {
                Box::new(Entity::try_from(ScriptEntity::from_value(ctx, value).ok()?).ok()?)
            }
            Kind::Option(inner) => Box::new(if value == Val::Null {
                DynamicEnum::new("None", DynamicVariant::Unit)
            } else {
                let mut some = DynamicTuple::default();
                some.insert_boxed(Self::to_reflect(ctx, inner, value)?);
                DynamicEnum::new("Some", DynamicVariant::Tuple(some))
            }),
            Kind::List(inner) => {
                let Val::Array(array) = value else {
                    return None;
                };
                let list: DynamicList = array
                    .0
                    .borrow()
                    .iter()
                    .map(|item| Self::to_reflect(ctx, inner, *item))
                    .collect::<Option<Vec<_>>>()?
                    .into_iter()
                    .collect();
                Box::new(list)
            }
            Kind::Type(id) => {
                let Val::Instance(instance) = value else {
                    return None;
                };
                let instance = instance.0.borrow();
                let values = instance.fields.as_slice();
                match info {
                    TypeInfo::Enum(variants) => {
                        let index = Self::variant_index(ctx, id, instance.struct_id)?;
                        let name = variants.variant_at(index)?.name();
                        let fields = Self::variant(ctx, Members::of(info, index)?, values)?;
                        Box::new(DynamicEnum::new(name, fields))
                    }
                    _ => match Self::variant(ctx, Members::of(info, 0)?, values)? {
                        DynamicVariant::Struct(fields) => Box::new(fields),
                        DynamicVariant::Tuple(fields) => Box::new(DynamicTupleStruct::from(fields)),
                        DynamicVariant::Unit => return None,
                    },
                }
            }
        })
    }

    /// The dynamic fields of a struct or enum variant.
    fn variant<'gc>(
        ctx: Ctx<'gc>,
        members: Members,
        values: &[Val<'gc>],
    ) -> Option<DynamicVariant> {
        Some(match members {
            Members::Unit => DynamicVariant::Unit,
            Members::Tuple(_) => {
                let mut fields = DynamicTuple::default();
                for ((_, member), value) in members.reflected().zip(values) {
                    fields.insert_boxed(Self::to_reflect(ctx, member, *value)?);
                }
                DynamicVariant::Tuple(fields)
            }
            Members::Named(_) => {
                let mut fields = DynamicStruct::default();
                for ((name, member), value) in members.reflected().zip(values) {
                    fields.insert_boxed(name?, Self::to_reflect(ctx, member, *value)?);
                }
                DynamicVariant::Struct(fields)
            }
        })
    }

    /// The `i`th field of a struct or variant (named fields work by position too).
    fn field<'a>(value: &ReflectRef<'a>, i: usize) -> Option<&'a dyn PartialReflect> {
        match value {
            ReflectRef::Struct(value) => value.field_at(i),
            ReflectRef::TupleStruct(value) => value.field(i),
            ReflectRef::Enum(value) => value.field_at(i),
            _ => None,
        }
    }

    /// Finds which variant a script enum value is.
    pub(crate) fn variant_index(ctx: Ctx<'_>, id: TypeId, struct_id: u32) -> Option<usize> {
        ctx.binding(id)?
            .variant_layout_ids
            .iter()
            .position(|layout| layout.index() as u32 == struct_id)
    }
}
