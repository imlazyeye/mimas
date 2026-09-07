use crate::{
    Instance, Val,
    conversion::{MimasType, TypeError},
    heap::Ctx,
};
use api::{ApiAdtKind, ApiVariantFields, Registry};
use shared::Ty;
use std::marker::PhantomData;

pub struct ApiAdtDescriptor {
    pub name: &'static str,
    pub module: &'static [&'static str],
    pub kind: ApiAdtKind,
    pub doc: &'static str,
    pub variants: Vec<ApiVariantShape>,
}

pub struct ApiVariantShape {
    pub name: String,
    pub doc: &'static str,
    pub fields: ApiVariantFields,
}

pub trait MimasAdt: 'static {
    fn descriptor(reg: &Registry) -> ApiAdtDescriptor;
}

/// Registration view of a `T` receiver that keeps the gc instance handle instead of converting
/// eagerly -- what `#[mimas] impl` uses for `&mut self` methods. `load` copies the fields out
/// into a `T`, `store` writes them back into the *same* instance, so mutations are visible to
/// mimas code and aliases of the instance.
pub struct InstanceOf<'gc, T>(pub Instance<'gc>, PhantomData<T>);

impl<'gc, T: MimasType<'gc>> MimasType<'gc> for InstanceOf<'gc, T> {
    fn mimas_ty(reg: &Registry) -> Option<Ty> {
        T::mimas_ty(reg)
    }
    fn from_value(_ctx: Ctx<'gc>, v: Val<'gc>) -> Result<Self, TypeError> {
        match v {
            Val::Instance(i) => Ok(Self(i, PhantomData)),
            other => Err(TypeError {
                expected: "instance".into(),
                got: format!("{other:?}"),
            }),
        }
    }
    fn into_value(self, _ctx: Ctx<'gc>) -> Val<'gc> {
        Val::Instance(self.0)
    }
}

impl<'gc, T: MimasType<'gc>> InstanceOf<'gc, T> {
    pub fn load(&self, ctx: Ctx<'gc>) -> T {
        T::from_value(ctx, Val::Instance(self.0)).expect("instance fields matched T's layout")
    }

    // note: if the method re-enters the vm before `store`, mimas code observes the
    // pre-call fields
    pub fn store(&self, ctx: Ctx<'gc>, value: T) {
        let Val::Instance(tmp) = value.into_value(ctx) else {
            unreachable!("an adt's into_value always builds an instance")
        };
        let fields = tmp.0.borrow().fields.clone();
        self.0.0.borrow_mut(&ctx).fields = fields;
    }
}
