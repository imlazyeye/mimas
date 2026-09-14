use std::{any::TypeId, cell::RefCell};

use bevy::{
    ecs::{
        message::{Message, MessageCursor, Messages},
        query::QueryBuilder,
    },
    platform::collections::HashMap,
    prelude::*,
    reflect::{FromReflect, Typed},
};

use crate::{
    bevy::{
        plugin::ScriptCtx, reflect::Catalog, types::Entity as ScriptEntity, writeback::HookScope,
    },
    vm::{Ctx, RtErr, Ty, Val, api::Api, conversion::MimasType},
};

/// Gives every reflected component and resource its script functions, and `Transform` its
/// constructor.
pub(crate) fn install(api: &mut Api, catalog: &Catalog) {
    let entity = api.ty_of::<ScriptEntity>();
    for (id, reflected) in catalog.stored() {
        let Some(ty) = Catalog::ty(api.library.registry(), id) else {
            continue;
        };
        let maybe = Ty::Option(Box::new(ty.clone()));
        if reflected.resource {
            api.add_assoc_described(ty.clone(), "get", vec![], maybe, move |ctx, _| {
                Ok(lend(ctx, id, None)?.unwrap_or(Val::Null))
            });
            api.add_assoc_described(
                ty.clone(),
                "insert",
                vec![ty],
                Ty::Unit,
                move |ctx, args| {
                    ctx.fixture::<HookScope>().forget(None, id);
                    let catalog = Catalog::of(ctx)?;
                    ctx.world(|world| catalog.insert(world, ctx, id, None, args[0]))??;
                    Ok(Val::Null)
                },
            );
            continue;
        }
        api.add_assoc_described(
            ty.clone(),
            "get",
            vec![entity.clone()],
            maybe.clone(),
            move |ctx, args| {
                Ok(lend(ctx, id, Some(entity_arg(ctx, args[0])?))?.unwrap_or(Val::Null))
            },
        );
        api.add_assoc_described(
            ty.clone(),
            "insert",
            vec![entity.clone(), ty.clone()],
            Ty::Unit,
            move |ctx, args| {
                let target = entity_arg(ctx, args[0])?;
                ctx.fixture::<HookScope>().forget(Some(target), id);
                let catalog = Catalog::of(ctx)?;
                ctx.world(|world| catalog.insert(world, ctx, id, Some(target), args[1]))??;
                Ok(Val::Null)
            },
        );
        api.add_assoc_described(
            ty.clone(),
            "remove",
            vec![entity.clone()],
            maybe,
            move |ctx, args| {
                let target = entity_arg(ctx, args[0])?;
                ctx.fixture::<HookScope>().forget(Some(target), id);
                let catalog = Catalog::of(ctx)?;
                Ok(ctx
                    .world(|world| catalog.remove(world, ctx, id, target))?
                    .unwrap_or(Val::Null))
            },
        );
        api.add_assoc_described(
            ty,
            "entities",
            vec![],
            Ty::Array(Box::new(entity.clone())),
            move |ctx, _| {
                let queries = ctx.fixture::<EntityQueries>();
                let entities: Vec<Entity> = ctx.world(|world| {
                    let Some(component) = world.components().get_valid_id(id) else {
                        return Vec::new();
                    };
                    let mut queries = queries.0.borrow_mut();
                    let query = queries.entry(id).or_insert_with(|| {
                        QueryBuilder::<Entity>::new(world)
                            .with_id(component)
                            .build()
                    });
                    query.iter(world).collect()
                })?;
                let entities = entities
                    .into_iter()
                    .map(|entity| ScriptEntity::from(entity).into_value(ctx))
                    .collect();
                Ok(Val::Array(ctx.new_array(entities)))
            },
        );
    }

    let transform = TypeId::of::<Transform>();
    if let Some(ty) = Catalog::ty(api.library.registry(), transform) {
        api.add_assoc_described(
            ty.clone(),
            "from_xyz",
            vec![Ty::Float, Ty::Float, Ty::Float],
            ty,
            |ctx, args| {
                let coordinate = |i: usize| args[i].as_float().unwrap_or_default() as f32;
                let value = Transform::from_xyz(coordinate(0), coordinate(1), coordinate(2));
                Catalog::of(ctx)?
                    .to_script(ctx, &value)
                    .ok_or_else(|| RtErr::Custom("`Transform` didn't convert".into()))
            },
        );
    }
}

/// Lends the script a copy of a component, or of a resource when `entity` is `None`. It's the
/// same loan whether it came from `get` or a hook parameter.
pub(crate) fn lend<'gc>(
    ctx: Ctx<'gc>,
    id: TypeId,
    entity: Option<Entity>,
) -> Result<Option<Val<'gc>>, RtErr> {
    let catalog = Catalog::of(ctx)?;
    ctx.fixture::<HookScope>().lend(ctx, entity, id, |world| {
        catalog.read(world, ctx, id, entity)
    })
}

/// Lets scripts read and write the message `T`, as `T::read()` and `T::write(message)`.
pub(crate) fn install_message<T: Message + FromReflect + Typed>(api: &mut Api) {
    let Some(ty) = Catalog::ty(api.library.registry(), TypeId::of::<T>()) else {
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
            let catalog = Catalog::of(ctx)?;
            let messages = ctx.world(|world| {
                world.resource_scope(|world, messages: Mut<Messages<T>>| {
                    let Ok(mut target) = world.get_entity_mut(entity) else {
                        return Some(Vec::new());
                    };
                    let mut cursor = target
                        .entry::<Cursor<T>>()
                        .or_insert_with(|| Cursor(MessageCursor::default()))
                        .into_mut();
                    cursor
                        .0
                        .read(&messages)
                        .map(|message| catalog.to_script(ctx, message))
                        .collect::<Option<Vec<_>>>()
                })
            })?;
            let messages =
                messages.ok_or_else(|| RtErr::Custom("a message didn't convert".into()))?;
            Ok(Val::Array(ctx.new_array(messages)))
        },
    );
    api.add_assoc_described(ty.clone(), "write", vec![ty], Ty::Unit, |ctx, args| {
        let message: T = Catalog::of(ctx)?
            .to_rust(ctx, args[0])
            .ok_or_else(|| RtErr::InvalidArgument("that value isn't the message type".into()))?;
        ctx.world(|world| {
            world.write_message(message);
        })?;
        Ok(Val::Null)
    });
}

/// Where a scripted entity's `read` picks up next time. It's kept on the entity, so it outlives
/// the script's Vm across reloads.
#[derive(Component)]
struct Cursor<T: Message>(MessageCursor<T>);

/// The queries behind `entities`, one per component. Building a query scans every archetype, so
/// each is built once per Vm.
#[derive(Default)]
struct EntityQueries(RefCell<HashMap<TypeId, QueryState<Entity>>>);

/// An entity argument, checked to be one.
fn entity_arg<'gc>(ctx: Ctx<'gc>, value: Val<'gc>) -> Result<Entity, RtErr> {
    ScriptEntity::from_value(ctx, value)?.try_into()
}
