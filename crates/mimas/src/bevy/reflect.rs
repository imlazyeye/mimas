use std::{any::TypeId, cell::OnceCell, sync::Arc};

use bevy::{
    ecs::reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    math::{Quat, Vec2, Vec3},
    platform::collections::{HashMap, HashSet},
    prelude::{Entity, World},
    reflect::{
        FromReflect, PartialReflect, ReflectRef, TypeInfo, TypeRegistry,
        enums::{DynamicEnum, DynamicVariant, VariantInfo},
        list::DynamicList,
        structs::DynamicStruct,
        tuple::DynamicTuple,
        tuple_struct::DynamicTupleStruct,
    },
};

use crate::{
    bevy::types::Entity as ScriptEntity,
    vm::{
        AdtBinding, ApiAdtKind, ApiVariantFields, Ctx, Fields, Registry, RtErr, Ty, Val,
        adt::{ApiAdtDescriptor, ApiVariantShape},
        api::Api,
        conversion::MimasType,
    },
};

/// How a value of some reflected type crosses into scripts, worked out once from its type info.
#[derive(Clone)]
pub(crate) enum Shape {
    Bool,
    Int(TypeId),
    Float(TypeId),
    Str,
    Vec2,
    Vec3,
    Quat,
    Entity,
    Option(&'static TypeInfo, Box<Shape>),
    List(&'static TypeInfo, Box<Shape>),
    Type(TypeId),
}

/// A reflected struct or enum that scripts can use, named after its Rust type.
pub(crate) struct Reflected {
    info: &'static TypeInfo,
    pub name: &'static str,
    module: &'static [&'static str],
    layout: Layout,
    /// Set for components, and for resources too, which Bevy stores as components.
    component: Option<ReflectComponent>,
    pub resource: bool,
}

/// The fields a reflected type carries, in declaration order.
enum Layout {
    Struct(Vec<(&'static str, Shape)>),
    TupleStruct(Vec<Shape>),
    Enum(Vec<(&'static str, Variant)>),
}

/// The fields of one enum variant.
enum Variant {
    Unit,
    Tuple(Vec<Shape>),
    Struct(Vec<(&'static str, Shape)>),
}

/// Every reflected type scripts can use, ordered so a type comes after the types its fields name.
/// It's built from Bevy's type registry whenever scripts compile, so a type that derives `Reflect`
/// reaches scripts without being registered for them.
pub(crate) struct Catalog {
    types: HashMap<TypeId, Reflected>,
    order: Vec<TypeId>,
}

/// Bookkeeping while a catalog is built: types that can't be represented, types being described
/// right now, and the names already taken.
struct Build<'a> {
    registry: &'a TypeRegistry,
    failed: HashSet<TypeId>,
    visiting: HashSet<TypeId>,
    names: HashSet<(&'static [&'static str], &'static str)>,
}

/// The catalog a Vm compiled against, reachable from its natives. A per-Vm fixture.
#[derive(Default)]
pub(crate) struct ScriptTypes(pub OnceCell<Arc<Catalog>>);

macro_rules! ints {
    ($($t:ty),*) => {
        fn is_int(id: TypeId) -> bool {
            $(id == TypeId::of::<$t>())||*
        }

        fn read_int(value: &dyn PartialReflect) -> Option<i64> {
            $(if let Some(n) = value.try_downcast_ref::<$t>() {
                return i64::try_from(*n).ok();
            })*
            None
        }

        fn boxed_int(id: TypeId, n: i64) -> Option<Box<dyn PartialReflect>> {
            $(if id == TypeId::of::<$t>() {
                return <$t>::try_from(n).ok().map(|n| Box::new(n) as Box<dyn PartialReflect>);
            })*
            None
        }
    };
}

ints!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl Catalog {
    /// Collects every component and resource scripts can represent, the `extra` types natives
    /// take, and whatever types their fields need.
    pub fn new(registry: &TypeRegistry, extra: &[TypeId]) -> Self {
        let mut catalog = Self {
            types: HashMap::default(),
            order: Vec::new(),
        };
        let mut roots: Vec<&'static TypeInfo> = registry
            .iter()
            .filter(|registration| {
                registration.data::<ReflectComponent>().is_some()
                    || extra.contains(&registration.type_id())
            })
            .map(|registration| registration.type_info())
            .collect();
        roots.sort_by_key(|info| info.type_path());
        let mut build = Build {
            registry,
            failed: HashSet::default(),
            visiting: HashSet::default(),
            names: HashSet::default(),
        };
        for info in roots {
            catalog.shape(&mut build, info);
        }
        catalog
    }

    /// The catalog the running script compiled against.
    pub fn of<'gc>(ctx: Ctx<'gc>) -> Result<&'gc Catalog, RtErr> {
        ctx.fixture::<ScriptTypes>()
            .0
            .get()
            .map(|catalog| &**catalog)
            .ok_or_else(|| RtErr::Custom("this Vm has no reflected types".into()))
    }

    /// The type scripts use for `id`, once it's registered.
    pub fn ty(registry: &Registry, id: TypeId) -> Option<Ty> {
        registry.get_id(id).map(|binding| Ty::Adt(binding.adt_id))
    }

    /// Every component and resource, in registration order.
    pub fn stored(&self) -> impl Iterator<Item = (TypeId, &Reflected)> {
        self.order
            .iter()
            .map(|id| (*id, &self.types[id]))
            .filter(|(_, reflected)| reflected.component.is_some())
    }

    fn shape(&mut self, build: &mut Build, info: &'static TypeInfo) -> Option<Shape> {
        let id = info.type_id();
        let simple = match id {
            _ if id == TypeId::of::<bool>() => Shape::Bool,
            _ if is_int(id) => Shape::Int(id),
            _ if id == TypeId::of::<f32>() || id == TypeId::of::<f64>() => Shape::Float(id),
            _ if id == TypeId::of::<String>() => Shape::Str,
            _ if id == TypeId::of::<Vec2>() => Shape::Vec2,
            _ if id == TypeId::of::<Vec3>() => Shape::Vec3,
            _ if id == TypeId::of::<Quat>() => Shape::Quat,
            _ if id == TypeId::of::<Entity>() => Shape::Entity,
            _ if self.types.contains_key(&id) => Shape::Type(id),
            _ => {
                // a type that names itself, directly or through others, isn't supported
                if build.failed.contains(&id) || !build.visiting.insert(id) {
                    return None;
                }
                let shape = self.describe(build, info);
                build.visiting.remove(&id);
                if shape.is_none() {
                    build.failed.insert(id);
                }
                return shape;
            }
        };
        Some(simple)
    }

    fn describe(&mut self, build: &mut Build, info: &'static TypeInfo) -> Option<Shape> {
        let path = info.type_path();
        let layout = match info {
            TypeInfo::Enum(options) if path.starts_with("core::option::Option<") => {
                let Some(VariantInfo::Tuple(some)) = options.variant("Some") else {
                    return None;
                };
                let inner = self.shape(build, some.field_at(0)?.type_info()?)?;
                return Some(Shape::Option(info, Box::new(inner)));
            }
            TypeInfo::List(list) => {
                let inner = self.shape(build, list.item_info()?)?;
                return Some(Shape::List(info, Box::new(inner)));
            }
            TypeInfo::Struct(fields) => Layout::Struct(
                fields
                    .iter()
                    .map(|field| Some((field.name(), self.shape(build, field.type_info()?)?)))
                    .collect::<Option<_>>()?,
            ),
            TypeInfo::TupleStruct(fields) => Layout::TupleStruct(
                fields
                    .iter()
                    .map(|field| self.shape(build, field.type_info()?))
                    .collect::<Option<_>>()?,
            ),
            TypeInfo::Enum(variants) => Layout::Enum(
                variants
                    .iter()
                    .map(|variant| {
                        let fields = match variant {
                            VariantInfo::Unit(_) => Variant::Unit,
                            VariantInfo::Tuple(fields) => Variant::Tuple(
                                fields
                                    .iter()
                                    .map(|field| self.shape(build, field.type_info()?))
                                    .collect::<Option<_>>()?,
                            ),
                            VariantInfo::Struct(fields) => Variant::Struct(
                                fields
                                    .iter()
                                    .map(|field| {
                                        Some((field.name(), self.shape(build, field.type_info()?)?))
                                    })
                                    .collect::<Option<_>>()?,
                            ),
                        };
                        Some((variant.name(), fields))
                    })
                    .collect::<Option<_>>()?,
            ),
            _ => return None,
        };
        let name = info.type_path_table().short_path();
        // Bevy's types come from its `bevy_*` crates and live in a `bevy` module, and the app's
        // own sit at the root
        let module: &'static [&'static str] = if path.starts_with("bevy_") {
            &["bevy"]
        } else {
            &[]
        };
        if name.contains('<') || !build.names.insert((module, name)) {
            return None;
        }
        let id = info.type_id();
        let registration = build.registry.get(id);
        self.types.insert(
            id,
            Reflected {
                info,
                name,
                module,
                layout,
                component: registration.and_then(|r| r.data::<ReflectComponent>().cloned()),
                resource: registration.is_some_and(|r| r.data::<ReflectResource>().is_some()),
            },
        );
        self.order.push(id);
        Some(Shape::Type(id))
    }

    /// Registers every type with the script's `Api`, skipping any a Rust registration already
    /// covers.
    pub fn install(&self, api: &mut Api) {
        for id in &self.order {
            if api.library.registry().get_id(*id).is_some() {
                continue;
            }
            let reflected = &self.types[id];
            api.add_adt_described(*id, |registry| {
                let named = |fields: &[(&'static str, Shape)]| {
                    ApiVariantFields::Named(
                        fields
                            .iter()
                            .map(|(name, shape)| (name.to_string(), ty(registry, shape)))
                            .collect(),
                    )
                };
                let positional = |fields: &[Shape]| {
                    ApiVariantFields::Tuple(
                        fields.iter().map(|shape| ty(registry, shape)).collect(),
                    )
                };
                let shape = |name: &str, fields| ApiVariantShape {
                    name: name.to_string(),
                    doc: "",
                    fields,
                };
                let (kind, variants) = match &reflected.layout {
                    Layout::Struct(fields) => (ApiAdtKind::Struct, vec![shape("@", named(fields))]),
                    Layout::TupleStruct(fields) => {
                        (ApiAdtKind::Struct, vec![shape("@", positional(fields))])
                    }
                    Layout::Enum(variants) => (
                        ApiAdtKind::Enum,
                        variants
                            .iter()
                            .map(|(name, variant)| {
                                let fields = match variant {
                                    Variant::Unit => ApiVariantFields::Unit,
                                    Variant::Tuple(fields) => positional(fields),
                                    Variant::Struct(fields) => named(fields),
                                };
                                shape(name, fields)
                            })
                            .collect(),
                    ),
                };
                ApiAdtDescriptor {
                    name: reflected.name,
                    module: reflected.module,
                    kind,
                    doc: "",
                    variants,
                }
            });
        }
    }

    /// The script's view of a Rust value whose type is in the catalog.
    pub fn to_script<'gc, T: PartialReflect>(&self, ctx: Ctx<'gc>, value: &T) -> Option<Val<'gc>> {
        self.to_val(ctx, &Shape::Type(TypeId::of::<T>()), value)
    }

    /// The Rust value a script's value stands for, when its type is in the catalog.
    pub fn to_rust<'gc, T: FromReflect>(&self, ctx: Ctx<'gc>, value: Val<'gc>) -> Option<T> {
        T::from_reflect(&*self.to_reflect(ctx, &Shape::Type(TypeId::of::<T>()), value)?)
    }

    /// The current component or resource, read into the Vm. `None` when it isn't there.
    pub fn read<'gc>(
        &self,
        world: &World,
        ctx: Ctx<'gc>,
        id: TypeId,
        entity: Option<Entity>,
    ) -> Option<Val<'gc>> {
        let reflected = self.types.get(&id)?;
        let holder = reflected.holder(world, id, entity)?;
        let value = reflected
            .component
            .as_ref()?
            .reflect(world.get_entity(holder).ok()?)?;
        self.to_val(ctx, &Shape::Type(id), value.as_partial_reflect())
    }

    /// Whether the entity has the component, or the world has the resource.
    pub fn has(&self, world: &World, id: TypeId, entity: Entity) -> bool {
        let Some(reflected) = self.types.get(&id) else {
            return false;
        };
        let Some(component) = &reflected.component else {
            return false;
        };
        reflected
            .holder(world, id, Some(entity))
            .and_then(|holder| world.get_entity(holder).ok())
            .is_some_and(|holder| component.contains(holder))
    }

    /// Writes a value a script edited back into the world. A value that comes back unchanged is
    /// left alone, so Bevy's change detection only fires for real changes.
    pub fn write<'gc>(
        &self,
        world: &mut World,
        ctx: Ctx<'gc>,
        id: TypeId,
        entity: Option<Entity>,
        value: Val<'gc>,
    ) -> Result<(), RtErr> {
        let (reflected, component) = self.stored_type(id)?;
        let edited = self.edited(ctx, reflected, id, value)?;
        // gone since the hook fetched it, so there's nothing to write back to
        let Some(holder) = reflected.holder(world, id, entity) else {
            return Ok(());
        };
        let mutable = world
            .components()
            .get_valid_id(id)
            .and_then(|component| world.components().get_info(component))
            .is_none_or(|info| info.mutable());
        let registry = world.resource::<AppTypeRegistry>().clone();
        let Ok(mut target) = world.get_entity_mut(holder) else {
            return Ok(());
        };
        let unchanged = component
            .reflect(&target)
            .and_then(|current| current.to_dynamic().reflect_partial_eq(&*edited))
            .unwrap_or(false);
        match (unchanged, mutable) {
            (true, _) => {}
            (false, true) => component.apply(&mut target, &*edited),
            (false, false) => component.insert(&mut target, &*edited, &registry.read()),
        }
        Ok(())
    }

    /// Adds or replaces a component, or replaces a resource.
    pub fn insert<'gc>(
        &self,
        world: &mut World,
        ctx: Ctx<'gc>,
        id: TypeId,
        entity: Option<Entity>,
        value: Val<'gc>,
    ) -> Result<(), RtErr> {
        let (reflected, component) = self.stored_type(id)?;
        let edited = self.edited(ctx, reflected, id, value)?;
        let holder = reflected.holder(world, id, entity).ok_or_else(|| {
            RtErr::Custom(format!(
                "there's no `{}` resource to replace",
                reflected.name
            ))
        })?;
        let registry = world.resource::<AppTypeRegistry>().clone();
        let mut target = world
            .get_entity_mut(holder)
            .map_err(|_| RtErr::InvalidArgument(format!("entity {holder} doesn't exist")))?;
        component.insert(&mut target, &*edited, &registry.read());
        Ok(())
    }

    /// Takes a component off an entity and hands it to the script.
    pub fn remove<'gc>(
        &self,
        world: &mut World,
        ctx: Ctx<'gc>,
        id: TypeId,
        entity: Entity,
    ) -> Option<Val<'gc>> {
        let component = self.types.get(&id)?.component.as_ref()?;
        let taken = component.take(&mut world.get_entity_mut(entity).ok()?)?;
        self.to_val(ctx, &Shape::Type(id), taken.as_partial_reflect())
    }

    fn stored_type(&self, id: TypeId) -> Result<(&Reflected, &ReflectComponent), RtErr> {
        self.types
            .get(&id)
            .and_then(|reflected| Some((reflected, reflected.component.as_ref()?)))
            .ok_or_else(|| RtErr::Custom("that type isn't a component or resource".into()))
    }

    fn edited<'gc>(
        &self,
        ctx: Ctx<'gc>,
        reflected: &Reflected,
        id: TypeId,
        value: Val<'gc>,
    ) -> Result<Box<dyn PartialReflect>, RtErr> {
        self.to_reflect(ctx, &Shape::Type(id), value)
            .ok_or_else(|| {
                RtErr::InvalidArgument(format!("that value isn't a `{}`", reflected.name))
            })
    }

    fn to_val<'gc>(
        &self,
        ctx: Ctx<'gc>,
        shape: &Shape,
        value: &dyn PartialReflect,
    ) -> Option<Val<'gc>> {
        Some(match shape {
            Shape::Bool => Val::Bool(*value.try_downcast_ref::<bool>()?),
            Shape::Int(_) => Val::Int(read_int(value)?),
            Shape::Float(_) => Val::Float(
                value
                    .try_downcast_ref::<f32>()
                    .map(|f| *f as f64)
                    .or_else(|| value.try_downcast_ref::<f64>().copied())?,
            ),
            Shape::Str => Val::Str(ctx.intern(value.try_downcast_ref::<String>()?)),
            Shape::Vec2 => (*value.try_downcast_ref::<Vec2>()?).into_value(ctx),
            Shape::Vec3 => (*value.try_downcast_ref::<Vec3>()?).into_value(ctx),
            Shape::Quat => (*value.try_downcast_ref::<Quat>()?).into_value(ctx),
            Shape::Entity => {
                ScriptEntity::from(*value.try_downcast_ref::<Entity>()?).into_value(ctx)
            }
            Shape::Option(_, inner) => {
                let ReflectRef::Enum(option) = value.reflect_ref() else {
                    return None;
                };
                match option.variant_name() {
                    "Some" => self.to_val(ctx, inner, option.field_at(0)?)?,
                    _ => Val::Null,
                }
            }
            Shape::List(_, inner) => {
                let ReflectRef::List(list) = value.reflect_ref() else {
                    return None;
                };
                let items = list
                    .iter()
                    .map(|item| self.to_val(ctx, inner, item))
                    .collect::<Option<Vec<_>>>()?;
                Val::Array(ctx.new_array(items))
            }
            Shape::Type(id) => {
                let reflected = self.types.get(id)?;
                let binding = binding(ctx, *id)?;
                let (layout, fields) = match (&reflected.layout, value.reflect_ref()) {
                    (Layout::Struct(fields), ReflectRef::Struct(value)) => (
                        binding.adt_id,
                        fields
                            .iter()
                            .map(|(name, shape)| self.to_val(ctx, shape, value.field(name)?))
                            .collect::<Option<Vec<_>>>()?,
                    ),
                    (Layout::TupleStruct(fields), ReflectRef::TupleStruct(value)) => (
                        binding.adt_id,
                        fields
                            .iter()
                            .enumerate()
                            .map(|(i, shape)| self.to_val(ctx, shape, value.field(i)?))
                            .collect::<Option<Vec<_>>>()?,
                    ),
                    (Layout::Enum(variants), ReflectRef::Enum(value)) => {
                        let index = variants
                            .iter()
                            .position(|(name, _)| *name == value.variant_name())?;
                        let fields = match &variants[index].1 {
                            Variant::Unit => Vec::new(),
                            Variant::Tuple(fields) => fields
                                .iter()
                                .enumerate()
                                .map(|(i, shape)| self.to_val(ctx, shape, value.field_at(i)?))
                                .collect::<Option<_>>()?,
                            Variant::Struct(fields) => fields
                                .iter()
                                .map(|(name, shape)| self.to_val(ctx, shape, value.field(name)?))
                                .collect::<Option<_>>()?,
                        };
                        (*binding.variant_layout_ids.get(index)?, fields)
                    }
                    _ => return None,
                };
                Val::Instance(ctx.new_instance(layout.index() as u32, Fields::new(fields)))
            }
        })
    }

    fn to_reflect<'gc>(
        &self,
        ctx: Ctx<'gc>,
        shape: &Shape,
        value: Val<'gc>,
    ) -> Option<Box<dyn PartialReflect>> {
        Some(match shape {
            Shape::Bool => Box::new(value.as_bool()?),
            Shape::Int(id) => boxed_int(*id, value.as_int()?)?,
            Shape::Float(id) if *id == TypeId::of::<f32>() => Box::new(value.as_float()? as f32),
            Shape::Float(_) => Box::new(value.as_float()?),
            Shape::Str => Box::new(value.as_str()?.as_str().to_string()),
            Shape::Vec2 => Box::new(Vec2::from_value(ctx, value).ok()?),
            Shape::Vec3 => Box::new(Vec3::from_value(ctx, value).ok()?),
            Shape::Quat => Box::new(Quat::from_value(ctx, value).ok()?),
            Shape::Entity => {
                Box::new(Entity::try_from(ScriptEntity::from_value(ctx, value).ok()?).ok()?)
            }
            Shape::Option(info, inner) => {
                let mut option = if value == Val::Null {
                    DynamicEnum::new("None", DynamicVariant::Unit)
                } else {
                    let mut some = DynamicTuple::default();
                    some.insert_boxed(self.to_reflect(ctx, inner, value)?);
                    DynamicEnum::new("Some", DynamicVariant::Tuple(some))
                };
                option.set_represented_type(Some(info));
                Box::new(option)
            }
            Shape::List(info, inner) => {
                let Val::Array(array) = value else {
                    return None;
                };
                let items: Vec<Val<'gc>> = array.0.borrow().clone();
                let mut list: DynamicList = items
                    .into_iter()
                    .map(|item| self.to_reflect(ctx, inner, item))
                    .collect::<Option<Vec<_>>>()?
                    .into_iter()
                    .collect();
                list.set_represented_type(Some(info));
                Box::new(list)
            }
            Shape::Type(id) => {
                let reflected = self.types.get(id)?;
                let Val::Instance(instance) = value else {
                    return None;
                };
                let (struct_id, values) = {
                    let instance = instance.0.borrow();
                    (instance.struct_id, instance.fields.as_slice().to_vec())
                };
                match &reflected.layout {
                    Layout::Struct(fields) => {
                        let mut value = self.named(ctx, fields, &values)?;
                        value.set_represented_type(Some(reflected.info));
                        Box::new(value)
                    }
                    Layout::TupleStruct(fields) => {
                        let mut value = DynamicTupleStruct::default();
                        for (shape, field) in fields.iter().zip(&values) {
                            value.insert_boxed(self.to_reflect(ctx, shape, *field)?);
                        }
                        value.set_represented_type(Some(reflected.info));
                        Box::new(value)
                    }
                    Layout::Enum(variants) => {
                        let index = binding(ctx, *id)?
                            .variant_layout_ids
                            .iter()
                            .position(|layout| layout.index() as u32 == struct_id)?;
                        let (name, variant) = variants.get(index)?;
                        let variant = match variant {
                            Variant::Unit => DynamicVariant::Unit,
                            Variant::Tuple(fields) => {
                                let mut tuple = DynamicTuple::default();
                                for (shape, field) in fields.iter().zip(&values) {
                                    tuple.insert_boxed(self.to_reflect(ctx, shape, *field)?);
                                }
                                DynamicVariant::Tuple(tuple)
                            }
                            Variant::Struct(fields) => {
                                DynamicVariant::Struct(self.named(ctx, fields, &values)?)
                            }
                        };
                        let mut value = DynamicEnum::new(*name, variant);
                        value.set_represented_type(Some(reflected.info));
                        Box::new(value)
                    }
                }
            }
        })
    }

    fn named<'gc>(
        &self,
        ctx: Ctx<'gc>,
        fields: &[(&'static str, Shape)],
        values: &[Val<'gc>],
    ) -> Option<DynamicStruct> {
        let mut value = DynamicStruct::default();
        for ((name, shape), field) in fields.iter().zip(values) {
            value.insert_boxed(*name, self.to_reflect(ctx, shape, *field)?);
        }
        Some(value)
    }
}

impl Reflected {
    /// The entity holding the value: the given one for a component, and the resource's own entity
    /// for a resource.
    fn holder(&self, world: &World, id: TypeId, entity: Option<Entity>) -> Option<Entity> {
        if self.resource {
            let component = world.components().get_valid_id(id)?;
            world.resource_entities().get(component)
        } else {
            entity
        }
    }
}

/// Where a registered type's instances live in the running Vm.
fn binding(ctx: Ctx<'_>, id: TypeId) -> Option<AdtBinding> {
    ctx.state().mimas_bindings.borrow().0.get(&id).cloned()
}

/// The type a shape has in scripts.
fn ty(registry: &Registry, shape: &Shape) -> Ty {
    let adt = |binding: Option<&AdtBinding>| {
        Ty::Adt(
            binding
                .expect("a field's type registers before the type that names it")
                .adt_id,
        )
    };
    match shape {
        Shape::Bool => Ty::Bool,
        Shape::Int(_) => Ty::Int,
        Shape::Float(_) => Ty::Float,
        Shape::Str => Ty::Str,
        Shape::Vec2 => adt(registry.get::<Vec2>()),
        Shape::Vec3 => adt(registry.get::<Vec3>()),
        Shape::Quat => adt(registry.get::<Quat>()),
        Shape::Entity => adt(registry.get::<ScriptEntity>()),
        Shape::Option(_, inner) => Ty::Option(Box::new(ty(registry, inner))),
        Shape::List(_, inner) => Ty::Array(Box::new(ty(registry, inner))),
        Shape::Type(id) => adt(registry.get_id(*id)),
    }
}
