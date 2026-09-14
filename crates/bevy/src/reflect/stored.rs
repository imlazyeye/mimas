use bevy::{
    ecs::{
        component::ComponentId,
        reflect::{AppTypeRegistry, ReflectComponent},
    },
    prelude::{Entity, World},
    reflect::{ReflectFromReflect, TypeInfo},
};

use super::{convert::Convert, kind::name};
use vm::{Ctx, RtErr, Val};

/// A component or resource scripts can use (Bevy stores resources as components too).
pub(crate) struct Stored {
    pub info: &'static TypeInfo,
    pub reflect: ReflectComponent,
    pub id: ComponentId,
    pub mutable: bool,
    pub resource: bool,
}

impl Stored {
    pub fn name(&self) -> &'static str {
        name(self.info)
    }

    /// Reads the current value into the Vm, or `None` if it's missing.
    pub fn read<'gc>(
        &self,
        world: &World,
        ctx: Ctx<'gc>,
        entity: Option<Entity>,
    ) -> Option<Val<'gc>> {
        let holder = self.holder(world, entity)?;
        let value = self.reflect.reflect(world.get_entity(holder).ok()?)?;
        let value = Convert::to_mimas(ctx, self.info, value.as_partial_reflect());
        Some(value.expect("a value of a catalog type always converts"))
    }

    /// Whether the entity has the component, or the world has the resource.
    pub fn has(&self, world: &World, entity: Entity) -> bool {
        self.holder(world, Some(entity))
            .and_then(|holder| world.get_entity(holder).ok())
            .is_some_and(|holder| self.reflect.contains(holder))
    }

    /// Writes a script's copy back into the world if it changed. This replaces the whole value
    /// (`apply` never shrinks lists).
    pub fn write<'gc>(
        &self,
        world: &mut World,
        ctx: Ctx<'gc>,
        entity: Option<Entity>,
        value: Val<'gc>,
    ) -> Result<(), RtErr> {
        let edited = Convert::to_reflect(ctx, self.info, value).ok_or_else(|| {
            RtErr::InvalidArgument(format!("that value isn't a `{}`", self.name()))
        })?;
        // it's been despawned or removed since the hook started
        let Some(mut target) = self
            .holder(world, entity)
            .and_then(|holder| world.get_entity_mut(holder).ok())
        else {
            return Ok(());
        };
        let Some(current) = self.reflect.reflect(&target) else {
            return Ok(());
        };
        if edited.reflect_partial_eq(current.as_partial_reflect()) == Some(true) {
            return Ok(());
        }
        let registry = target.world().resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        if !self.mutable {
            // immutable components have to be inserted again
            self.reflect.insert(&mut target, &*edited, &registry);
            return Ok(());
        }
        let replacement = registry
            .get_type_data::<ReflectFromReflect>(self.info.type_id())
            .unwrap_or_else(|| {
                panic!(
                    "scripts can't write `{}` back, since it doesn't reflect `FromReflect`",
                    self.name()
                )
            })
            .from_reflect(&*edited)
            .expect("a value converted for a type fits it");
        self.reflect
            .reflect_mut(&mut target)
            .expect("the component was just read")
            .set(replacement)
            .expect("the replacement is the component's own type");
        Ok(())
    }

    /// Adds or replaces a component, or replaces a resource.
    pub fn insert<'gc>(
        &self,
        world: &mut World,
        ctx: Ctx<'gc>,
        entity: Option<Entity>,
        value: Val<'gc>,
    ) -> Result<(), RtErr> {
        let edited = Convert::to_reflect(ctx, self.info, value).ok_or_else(|| {
            RtErr::InvalidArgument(format!("that value isn't a `{}`", self.name()))
        })?;
        let holder = self.holder(world, entity).ok_or_else(|| {
            RtErr::Custom(format!("there's no `{}` resource to replace", self.name()))
        })?;
        let registry = world.resource::<AppTypeRegistry>().clone();
        let mut target = world
            .get_entity_mut(holder)
            .map_err(|_| RtErr::InvalidArgument(format!("entity {holder} doesn't exist")))?;
        self.reflect.insert(&mut target, &*edited, &registry.read());
        Ok(())
    }

    /// Removes a component and returns it to the script.
    pub fn remove<'gc>(
        &self,
        world: &mut World,
        ctx: Ctx<'gc>,
        entity: Entity,
    ) -> Option<Val<'gc>> {
        let taken = self.reflect.take(&mut world.get_entity_mut(entity).ok()?)?;
        Convert::to_mimas(ctx, self.info, taken.as_partial_reflect())
    }

    /// The entity the value lives on (resources get an entity of their own).
    fn holder(&self, world: &World, entity: Option<Entity>) -> Option<Entity> {
        if self.resource {
            world.resource_entities().get(self.id)
        } else {
            entity
        }
    }
}
