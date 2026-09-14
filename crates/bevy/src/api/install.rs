use std::sync::Arc;

use bevy::prelude::{Resource, Time};

use crate::{
    hooks::{Hook, register},
    plugin::ScriptCtx,
    reflect::Catalog,
};
use macros::native;
use vm::{Ctx, RtErr, Ty, Val, api::Api};

/// Registers everything under the `bevy` module.
pub(crate) fn install(api: &mut Api, catalog: &Arc<Catalog>) {
    super::ecs::install_entity(api);
    catalog.install(api);
    super::ecs::install(api, catalog);
    super::input::install(api);

    // `bevy::update(f)` and friends
    let mut bevy = api.module("bevy");
    for hook in Hook::ALL {
        let catalog = catalog.clone();
        bevy.add_described(hook.name(), vec![None], Ty::Unit, move |ctx, args| {
            register(ctx, &catalog, hook, args[0])?;
            Ok(Val::Null)
        });
    }

    let mut time = api.module("bevy::time");
    time.add(delta);
    time.add(elapsed);

    let mut log = api.module("bevy::log");
    log.add(info);
    log.add(warn);
    log.add(error);
}

/// Seconds since the last frame, or the fixed step inside `fixed_update`.
#[native]
fn delta(ctx: Ctx) -> Result<f32, RtErr> {
    resource::<Time, _>(ctx, Time::delta_secs)
}

/// Seconds since the app started.
#[native]
fn elapsed(ctx: Ctx) -> Result<f32, RtErr> {
    resource::<Time, _>(ctx, Time::elapsed_secs)
}

/// Logs at info level.
#[native]
fn info(message: &str) {
    bevy::log::info!("{message}");
}

/// Logs at warn level.
#[native]
fn warn(message: &str) {
    bevy::log::warn!("{message}");
}

/// Logs at error level.
#[native]
fn error(message: &str) {
    bevy::log::error!("{message}");
}

/// Reads a resource, or errors if the world doesn't have it.
pub(crate) fn resource<R: Resource, T>(ctx: Ctx, f: impl FnOnce(&R) -> T) -> Result<T, RtErr> {
    ctx.world(|world| world.get_resource::<R>().map(f))
        .ok_or_else(|| {
            RtErr::Custom(format!(
                "no `{}` resource in the world",
                std::any::type_name::<R>()
            ))
        })
}
