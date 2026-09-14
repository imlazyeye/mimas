use std::{any::TypeId, cell::RefCell, sync::Arc};

use bevy::{
    ecs::{
        component::ComponentId,
        message::{Message, MessageCursor, Messages},
        query::QueryBuilder,
    },
    platform::collections::HashMap,
    prelude::*,
    reflect::{FromReflect, Typed},
};

use crate::{
    plugin::ScriptCtx,
    reflect::{Catalog, Convert},
    script::{MimasScript, Script},
    types::Entity as ScriptEntity,
    writeback::HookScope,
};
use macros::native;
use vm::{Ctx, RtErr, Ty, Val, api::Api, conversion::MimasType};

/// Registers `bevy::Entity` and `bevy::ecs`. Has to run before the catalog (reflected types can
/// have `Entity` fields).
pub(crate) fn install_entity(api: &mut Api) {
    api.module("bevy").add_adt::<ScriptEntity>();
    api.add_assoc_of::<ScriptEntity, _, _>("spawn", spawn);
    api.add_method(despawn);
    api.add_method(exists);
    let mut ecs = api.module("bevy::ecs");
    ecs.add(me);
    ecs.add(attach_script);
}

/// Adds `get`, `insert`, `remove`, and `entities` to components (resources just get `get` and
/// `insert`).
pub(crate) fn install(api: &mut Api, catalog: &Arc<Catalog>) {
    fn entity_arg<'gc>(ctx: Ctx<'gc>, value: Val<'gc>) -> Result<Entity, RtErr> {
        ScriptEntity::from_value(ctx, value)?.try_into()
    }

    /// The entity argument, or `None` for a resource.
    fn holder_arg<'gc>(
        ctx: Ctx<'gc>,
        args: &[Val<'gc>],
        resource: bool,
    ) -> Result<Option<Entity>, RtErr> {
        if resource {
            return Ok(None);
        }
        entity_arg(ctx, args[0]).map(Some)
    }

    let entity = api.ty_of::<ScriptEntity>();
    for (id, stored) in catalog.stored() {
        let ty = api
            .library
            .registry()
            .ty_of_id(id)
            .expect("the catalog installed every component and resource");
        let maybe = Ty::Option(Box::new(ty.clone()));
        let resource = stored.resource;
        let holder = if resource {
            vec![]
        } else {
            vec![entity.clone()]
        };
        api.add_assoc_described(ty.clone(), "get", holder.clone(), maybe.clone(), {
            let catalog = catalog.clone();
            move |ctx, args| {
                let holder = holder_arg(ctx, args, resource)?;
                let lent = ctx.fixture::<HookScope>().lend(ctx, &catalog, holder, id)?;
                Ok(lent.unwrap_or(Val::Null))
            }
        });
        api.add_assoc_described(
            ty.clone(),
            "insert",
            [holder, vec![ty.clone()]].concat(),
            Ty::Unit,
            {
                let catalog = catalog.clone();
                move |ctx, args| {
                    let holder = holder_arg(ctx, args, resource)?;
                    let value = args[args.len() - 1];
                    ctx.fixture::<HookScope>().forget(holder, id);
                    ctx.world(|world| catalog.get(id).insert(world, ctx, holder, value))?;
                    Ok(Val::Null)
                }
            },
        );
        if resource {
            continue;
        }
        api.add_assoc_described(ty.clone(), "remove", vec![entity.clone()], maybe, {
            let catalog = catalog.clone();
            move |ctx, args| {
                let target = entity_arg(ctx, args[0])?;
                ctx.fixture::<HookScope>().forget(Some(target), id);
                let removed = ctx.world(|world| catalog.get(id).remove(world, ctx, target));
                Ok(removed.unwrap_or(Val::Null))
            }
        });
        let component = stored.id;
        api.add_assoc_described(
            ty,
            "entities",
            vec![],
            Ty::Array(Box::new(entity.clone())),
            move |ctx, _| {
                let queries = ctx.fixture::<EntityQueries>();
                let entities = ctx.world(|world| {
                    let mut queries = queries.0.borrow_mut();
                    let query = queries.entry(component).or_insert_with(|| {
                        QueryBuilder::<Entity>::new(world)
                            .with_id(component)
                            .build()
                    });
                    query
                        .iter(world)
                        .map(|entity| ScriptEntity::from(entity).into_value(ctx))
                        .collect()
                });
                Ok(Val::Array(ctx.new_array(entities)))
            },
        );
    }
}

/// The entity whose hook is running.
#[native]
fn me(ctx: Ctx) -> Result<ScriptEntity, RtErr> {
    Ok(ctx.entity()?.into())
}

/// Gives the entity the script at `path` (relative to `assets/`), replacing any script it
/// already had.
#[native]
fn attach_script(ctx: Ctx, entity: ScriptEntity, path: &str) -> Result<(), RtErr> {
    let entity: Entity = entity.try_into()?;
    ctx.world(|world| {
        let handle = world
            .resource::<AssetServer>()
            .load::<MimasScript>(path.to_string());
        world
            .get_entity_mut(entity)
            .map_err(|_| RtErr::InvalidArgument(format!("entity {entity} doesn't exist")))?
            .insert(Script(handle));
        Ok(())
    })
}

/// Spawns an empty entity.
#[native]
fn spawn(ctx: Ctx) -> ScriptEntity {
    ctx.world(|world| world.spawn_empty().id().into())
}

/// Despawns the entity, and returns `false` if it was already gone.
#[native]
fn despawn(ctx: Ctx, entity: ScriptEntity) -> Result<bool, RtErr> {
    let entity: Entity = entity.try_into()?;
    Ok(ctx.world(|world| world.despawn(entity)))
}

/// Whether the entity is still alive.
#[native]
fn exists(ctx: Ctx, entity: ScriptEntity) -> Result<bool, RtErr> {
    let entity: Entity = entity.try_into()?;
    Ok(ctx.world(|world| world.get_entity(entity).is_ok()))
}

/// Adds `T::read()` and `T::write(message)` for the message `T`.
pub(crate) fn install_message<T: Message + FromReflect + Typed>(api: &mut Api) {
    let Some(ty) = api.library.registry().ty_of_id(TypeId::of::<T>()) else {
        error!(
            "scripts can't use `{}`, since a field of it has no mimas type",
            T::type_path()
        );
        return;
    };
    api.add_assoc_described(
        ty.clone(),
        "read",
        vec![],
        Ty::Array(Box::new(ty.clone())),
        |ctx, _| {
            let entity = ctx.entity()?;
            let messages = ctx.world(|world| {
                world.resource_scope(|world, messages: Mut<Messages<T>>| {
                    let Ok(mut target) = world.get_entity_mut(entity) else {
                        return Vec::new();
                    };
                    let mut cursor = target
                        .entry::<Cursor<T>>()
                        .or_insert_with(|| Cursor(MessageCursor::default()))
                        .into_mut();
                    cursor
                        .0
                        .read(&messages)
                        .map(|message| {
                            Convert::to_mimas(ctx, T::type_info(), message)
                                .expect("a value of a catalog type always converts")
                        })
                        .collect()
                })
            });
            Ok(Val::Array(ctx.new_array(messages)))
        },
    );
    api.add_assoc_described(ty.clone(), "write", vec![ty], Ty::Unit, |ctx, args| {
        let message: T = Convert::to_rust(ctx, args[0])
            .ok_or_else(|| RtErr::InvalidArgument("that value isn't the message type".into()))?;
        ctx.world(|world| {
            world.write_message(message);
        });
        Ok(Val::Null)
    });
}

/// Which messages an entity has already read. Lives on the entity to survive reloads.
#[derive(Component)]
struct Cursor<T: Message>(MessageCursor<T>);

/// Cached queries for `entities` (building one scans every archetype).
#[derive(Default)]
struct EntityQueries(RefCell<HashMap<ComponentId, QueryState<Entity>>>);
