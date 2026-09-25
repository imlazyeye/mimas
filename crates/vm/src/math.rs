use std::any::TypeId;

use glam::{Quat, Vec2, Vec3};

use crate::{
    ApiAdtKind, ApiVariantFields, Ctx, Fields, Registry, Ty, Val,
    adt::{ApiAdtDescriptor, ApiVariantShape, MimasAdt},
    conversion::{MimasType, TypeError},
};

macro_rules! glam_struct {
    ($ty:ident, $doc:literal, $new:path, $($field:ident),+) => {
        impl MimasAdt for $ty {
            fn descriptor(_: &Registry) -> ApiAdtDescriptor {
                ApiAdtDescriptor {
                    name: stringify!($ty),
                    module: &[],
                    kind: ApiAdtKind::Struct,
                    doc: $doc,
                    variants: vec![ApiVariantShape {
                        name: "@".to_string(),
                        doc: "",
                        fields: ApiVariantFields::Named(vec![
                            $((stringify!($field).to_string(), Ty::Float)),+
                        ]),
                    }],
                }
            }
        }

        impl<'gc> MimasType<'gc> for $ty {
            fn mimas_ty(reg: &Registry) -> Option<Ty> {
                Some(
                    reg.ty_of_id(TypeId::of::<Self>())
                        .expect(concat!("`", stringify!($ty), "` isn't registered with mimas")),
                )
            }

            fn from_value(ctx: Ctx<'gc>, value: Val<'gc>) -> Result<Self, TypeError> {
                let mismatch = || TypeError {
                    expected: stringify!($ty).into(),
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
                    <f32 as MimasType>::from_value(ctx, fields.next().ok_or_else(|| {
                        TypeError { expected: stringify!($field).into(), got: "<empty>".into() }
                    })?)?
                ),+))
            }

            fn into_value(self, ctx: Ctx<'gc>) -> Val<'gc> {
                let id = ctx
                    .binding(TypeId::of::<Self>())
                    .expect(concat!("`", stringify!($ty), "` isn't registered with mimas"))
                    .adt_id;
                let fields = vec![$(Val::Float(self.$field as f64)),+];
                Val::Instance(ctx.new_instance(id.index() as u32, Fields::new(fields)))
            }
        }
    };
}

glam_struct!(
    Vec2,
    "A 2D vector with `float` components `x` and `y`. Create one with `Vec2::new(x, y)` or a \
     struct literal such as `Vec2 { x = 1.0, y = 2.0 }`.\n\n\
     The components are stored as 32-bit floats, and can read back slightly different from the \
     value that was set (`0.1` reads back as `0.10000000149011612`).",
    Vec2::new,
    x,
    y
);
glam_struct!(
    Vec3,
    "A 3D vector with `float` components `x`, `y`, and `z`. Create one with a struct literal \
     such as `Vec3 { x = 0.0, y = 1.0, z = 0.0 }`.\n\n\
     Like `Vec2`, the components are stored as 32-bit floats.",
    Vec3::new,
    x,
    y,
    z
);
glam_struct!(
    Quat,
    "A rotation in 3D space, stored as a quaternion with `float` components `x`, `y`, `z`, and \
     `w`. Scripts usually create one with `Quat::from_rotation_z` and pass it to the host, such \
     as for the rotation of a Bevy transform.",
    Quat::from_xyzw,
    x,
    y,
    z,
    w
);
