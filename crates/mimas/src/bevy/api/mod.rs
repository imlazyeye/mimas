use std::sync::Arc;

use bevy::prelude::{Resource, Time};

use crate::{
    bevy::{
        plugin::ScriptCtx,
        reflect::{Catalog, ScriptTypes},
    },
    native,
    vm::{Ctx, RtErr, api::Api},
};

mod ecs;
mod entity;
mod input;

pub(crate) use ecs::{install_message, lend};

/// Registers everything a script gets besides the standard library: the reflected types and their
/// functions, entities, input, and the `time` and `log` modules.
pub(crate) fn install(api: &mut Api, catalog: &Arc<Catalog>) {
    let _ = api.ctx.fixture::<ScriptTypes>().0.set(catalog.clone());
    entity::install(api);
    catalog.install(api);
    ecs::install(api, catalog);
    input::install(api);

    let mut time = api.module("time");
    time.add(delta);
    time.add(elapsed);

    let mut log = api.module("log");
    log.add(info);
    log.add(warn);
    log.add(error);
}

/// Reads a resource through its Rust type, faulting when the world doesn't have one.
fn resource<R: Resource, T>(ctx: Ctx, f: impl FnOnce(&R) -> T) -> Result<T, RtErr> {
    ctx.world(|world| world.get_resource::<R>().map(f))?
        .ok_or_else(|| {
            RtErr::Custom(format!(
                "no `{}` resource in the world",
                std::any::type_name::<R>()
            ))
        })
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
