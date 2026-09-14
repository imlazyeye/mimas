use crate::{
    bevy::{
        plugin::ScriptCtx,
        script::{MimasScript, Script},
        types::Entity,
    },
    native,
    vm::{Ctx, RtErr, api::Api},
};

/// Registers `bevy::Entity` with its `spawn`, `despawn` and `exists`, and the `script` module.
pub(crate) fn install(api: &mut Api) {
    api.module("bevy").add_adt::<Entity>();
    api.add_assoc_of::<Entity, _, _>("spawn", spawn);
    api.add_method(despawn);
    api.add_method(exists);
    let mut script = api.module("script");
    script.add(entity);
    script.add(attach);
}

/// The entity whose hook is running.
#[native]
fn entity(ctx: Ctx) -> Result<Entity, RtErr> {
    Ok(ctx.entity()?.into())
}

/// Gives the entity a script of its own, loaded from `path` under `assets/`. One it already runs
/// is stopped first.
#[native]
fn attach(ctx: Ctx, entity: Entity, path: &str) -> Result<(), RtErr> {
    let entity: bevy::prelude::Entity = entity.try_into()?;
    ctx.world(|world| {
        let handle = world
            .resource::<bevy::asset::AssetServer>()
            .load::<MimasScript>(path.to_string());
        world
            .get_entity_mut(entity)
            .map_err(|_| RtErr::InvalidArgument(format!("entity {entity} doesn't exist")))?
            .insert(Script(handle));
        Ok(())
    })?
}

/// Spawns an empty entity.
#[native]
fn spawn(ctx: Ctx) -> Result<Entity, RtErr> {
    ctx.world(|world| world.spawn_empty().id().into())
}

/// Despawns the entity, and returns `false` if it was already gone.
#[native]
fn despawn(ctx: Ctx, entity: Entity) -> Result<bool, RtErr> {
    let entity: bevy::prelude::Entity = entity.try_into()?;
    ctx.world(|world| world.despawn(entity))
}

/// Whether the entity is still alive.
#[native]
fn exists(ctx: Ctx, entity: Entity) -> Result<bool, RtErr> {
    let entity: bevy::prelude::Entity = entity.try_into()?;
    ctx.world(|world| world.get_entity(entity).is_ok())
}
