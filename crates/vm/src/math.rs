use std::any::TypeId;

use glam::{I64Vec2, I64Vec3, Quat, Vec2, Vec3};

use crate::{
    ApiAdtKind, ApiVariantFields, Ctx, Fields, Registry, Ty, Val,
    adt::{ApiAdtDescriptor, ApiVariantShape, MimasAdt},
    conversion::{MimasType, TypeError},
};

macro_rules! glam_struct {
    ($ty:ident, $name:literal, $scalar:ident, $field_ty:ident, $doc:literal, $new:path, $($field:ident),+) => {
        impl MimasAdt for $ty {
            fn descriptor(_: &Registry) -> ApiAdtDescriptor {
                ApiAdtDescriptor {
                    name: $name,
                    module: &[],
                    kind: ApiAdtKind::Struct,
                    doc: $doc,
                    variants: vec![ApiVariantShape {
                        name: "@".to_string(),
                        doc: "",
                        fields: ApiVariantFields::Named(vec![
                            $((stringify!($field).to_string(), Ty::$field_ty)),+
                        ]),
                    }],
                }
            }
        }

        impl<'gc> MimasType<'gc> for $ty {
            fn mimas_ty(reg: &Registry) -> Option<Ty> {
                Some(
                    reg.ty_of_id(TypeId::of::<Self>())
                        .expect(concat!("`", $name, "` isn't registered with mimas")),
                )
            }

            fn from_value(ctx: Ctx<'gc>, value: Val<'gc>) -> Result<Self, TypeError> {
                let mismatch = || TypeError {
                    expected: $name.into(),
                    got: format!("{value:?}"),
                };
                let Val::Instance(instance) = value else {
                    return Err(mismatch());
                };
                let id = ctx.binding(TypeId::of::<Self>()).map(|b| b.adt_id);
                let fields = {
                    let instance = instance.0.borrow();
                    if id.is_none_or(|id| id.index() as u32 != instance.struct_id) {
                        return Err(mismatch());
                    }
                    instance.fields.clone()
                };
                let mut fields = fields.into_iter();
                Ok($new($(
                    <$scalar as MimasType>::from_value(ctx, fields.next().ok_or_else(|| {
                        TypeError { expected: stringify!($field).into(), got: "<empty>".into() }
                    })?)?
                ),+))
            }

            fn into_value(self, ctx: Ctx<'gc>) -> Val<'gc> {
                let id = ctx
                    .binding(TypeId::of::<Self>())
                    .expect(concat!("`", $name, "` isn't registered with mimas"))
                    .adt_id;
                let fields = vec![$(self.$field.into_value(ctx)),+];
                Val::Instance(ctx.new_instance(id.index() as u32, Fields::new(fields)))
            }
        }
    };
}

glam_struct!(
    Vec2,
    "Vec2",
    f32,
    Float,
    "A 2D vector with `float` components `x` and `y`. Create one with `vec2(x, y)`, \
     `Vec2::new(x, y)` or a struct literal such as `Vec2 { x = 1.0, y = 2.0 }`.\n\n\
     The components are stored as 32-bit floats, and can read back slightly different from the \
     value that was set (`0.1` reads back as `0.10000000149011612`).",
    Vec2::new,
    x,
    y
);
glam_struct!(
    Vec3,
    "Vec3",
    f32,
    Float,
    "A 3D vector with `float` components `x`, `y`, and `z`. Create one with `vec3(x, y, z)` or \
     a struct literal such as `Vec3 { x = 0.0, y = 1.0, z = 0.0 }`.\n\n\
     Like `Vec2`, the components are stored as 32-bit floats.",
    Vec3::new,
    x,
    y,
    z
);
glam_struct!(
    Quat,
    "Quat",
    f32,
    Float,
    "A rotation in 3D space, stored as a quaternion with `float` components `x`, `y`, `z`, and \
     `w`. Scripts usually create one with `Quat::from_rotation_z` and pass it to the host, such \
     as for the rotation of a Bevy transform.",
    Quat::from_xyzw,
    x,
    y,
    z,
    w
);
glam_struct!(
    I64Vec2,
    "IVec2",
    i64,
    Int,
    "A 2D vector with `int` components `x` and `y`, for things counted in whole steps such as \
     pixels and grid cells. Create one with `ivec2(x, y)`, `IVec2::new(x, y)` or a struct literal \
     such as `IVec2 { x = 1, y = 2 }`.\n\n\
     `Vec2.to_ivec2` and `IVec2.to_vec2` convert between the two.",
    I64Vec2::new,
    x,
    y
);
glam_struct!(
    I64Vec3,
    "IVec3",
    i64,
    Int,
    "A 3D vector with `int` components `x`, `y`, and `z`. Create one with `ivec3(x, y, z)`, \
     `IVec3::new(x, y, z)` or a struct literal such as `IVec3 { x = 0, y = 1, z = 0 }`.",
    I64Vec3::new,
    x,
    y,
    z
);
