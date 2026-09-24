use std::any::TypeId;

use bevy::{
    ecs::reflect::{AppTypeRegistry, ReflectComponent},
    platform::collections::HashMap,
    prelude::World,
    reflect::{TypeInfo, std_traits::ReflectDefault},
};

use super::{
    builder::{Builder, module},
    convert::Convert,
    kind::{Kind, Members, kind, name},
    stored::Stored,
};
use crate::types::Entity as ScriptEntity;
use vm::{
    ApiAdtKind, ApiVariantFields, Registry, Ty,
    adt::{ApiAdtDescriptor, ApiVariantShape},
    api::Api,
    conversion::MimasType,
};

/// Every reflected type scripts can use, built from Bevy's type registry. `order` lists field
/// types before the types that use them.
pub(crate) struct Catalog {
    pub types: HashMap<TypeId, Reflected>,
    pub order: Vec<TypeId>,
}

impl Catalog {
    /// Builds the catalog from every component and resource, plus the `extra` types (like
    /// messages).
    pub fn new(world: &mut World, extra: &[TypeId]) -> Self {
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        let mut roots: Vec<&'static TypeInfo> = registry
            .iter()
            .filter(|registration| {
                registration.data::<ReflectComponent>().is_some()
                    || extra.contains(&registration.type_id())
            })
            .map(|registration| registration.type_info())
            .collect();
        roots.sort_by_key(|info| info.type_path());
        let mut builder = Builder::new(&registry, world);
        for info in roots {
            builder.accept(info);
        }
        builder.catalog
    }

    /// Registers every type with the `Api`, plus `T::default()` for types that reflect `Default`.
    pub fn install(&self, api: &mut Api) {
        /// The script type for `info`.
        fn ty(registry: &Registry, info: &'static TypeInfo) -> Ty {
            const REGISTERED: &str = "a field's type registers before the type that names it";
            match kind(info).expect("the catalog only takes types scripts can hold") {
                Kind::Bool => Ty::Bool,
                Kind::Int(_) => Ty::Int,
                Kind::F32 | Kind::F64 => Ty::Float,
                Kind::Str => Ty::Str,
                Kind::Entity => ScriptEntity::mimas_ty(registry).expect(REGISTERED),
                Kind::Option(inner) => Ty::Option(Box::new(ty(registry, inner))),
                Kind::List(inner) => Ty::Array(Box::new(ty(registry, inner))),
                Kind::Vec2 | Kind::Vec3 | Kind::Quat | Kind::Type(_) => {
                    registry.ty_of_id(info.type_id()).expect(REGISTERED)
                }
            }
        }

        const TAKEN: &str = "the catalog only takes structs and enums";
        for id in &self.order {
            let reflected = &self.types[id];
            let info = reflected.info;
            let binding = api.add_adt_described(*id, |registry| {
                let variant = |name: &str, doc: &'static str, members: Members| ApiVariantShape {
                    name: name.to_string(),
                    doc,
                    fields: match members {
                        Members::Unit => ApiVariantFields::Unit,
                        Members::Tuple(_) => ApiVariantFields::Tuple(
                            members
                                .reflected()
                                .map(|(_, member)| ty(registry, member))
                                .collect(),
                        ),
                        Members::Named(_) => ApiVariantFields::Named(
                            members
                                .reflected()
                                .map(|(name, member)| {
                                    (name.unwrap_or_default().to_string(), ty(registry, member))
                                })
                                .collect(),
                        ),
                    },
                };
                let (kind, variants) = match info {
                    TypeInfo::Enum(variants) => (
                        ApiAdtKind::Enum,
                        variants
                            .iter()
                            .enumerate()
                            .map(|(i, v)| {
                                variant(
                                    v.name(),
                                    v.docs().unwrap_or_default(),
                                    Members::of(info, i).expect(TAKEN),
                                )
                            })
                            .collect(),
                    ),
                    _ => (
                        ApiAdtKind::Struct,
                        vec![variant("@", "", Members::of(info, 0).expect(TAKEN))],
                    ),
                };
                ApiAdtDescriptor {
                    name: name(info),
                    module: module(info),
                    kind,
                    doc: info.docs().unwrap_or_default(),
                    variants,
                }
            });
            if let Some(default) = reflected.default.clone() {
                let ty = Ty::Adt(binding.adt_id);
                api.add_assoc_described(ty.clone(), "default", vec![], ty, move |ctx, _| {
                    let value =
                        Convert::to_mimas(ctx, info, default.default().as_partial_reflect());
                    Ok(value.expect("a value of a catalog type always converts"))
                });
            }
        }
    }

    /// Looks up a component or resource. Panics if `id` isn't one.
    pub fn get(&self, id: TypeId) -> &Stored {
        self.types[&id]
            .stored
            .as_ref()
            .expect("only components and resources are looked up")
    }

    /// Every component and resource, in registration order.
    pub fn stored(&self) -> impl Iterator<Item = (TypeId, &Stored)> {
        self.order
            .iter()
            .filter_map(|id| Some((*id, self.types[id].stored.as_ref()?)))
    }
}

/// A struct or enum in the catalog.
pub(crate) struct Reflected {
    pub info: &'static TypeInfo,
    pub stored: Option<Stored>,
    pub default: Option<ReflectDefault>,
}
