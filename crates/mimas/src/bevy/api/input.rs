use std::{any::TypeId, cell::RefCell, hash::Hash};

use bevy::{
    camera::Camera,
    ecs::query::QueryState,
    input::{ButtonInput, keyboard::KeyCode, mouse::MouseButton},
    math::Vec2,
    prelude::{GlobalTransform, With},
    reflect::FromReflect,
    window::{PrimaryWindow, Window},
};

use crate::{
    bevy::{plugin::ScriptCtx, reflect::Catalog},
    native,
    vm::{Ctx, RtErr, RtResult, Ty, Val, api::Api},
};

use super::resource;

/// Registers the `input` module. Keys and mouse buttons are Bevy's own enums, so a misspelled
/// `bevy::KeyCode` variant fails when the script loads.
pub(crate) fn install(api: &mut Api) {
    let key = Catalog::ty(api.library.registry(), TypeId::of::<KeyCode>());
    let mouse = Catalog::ty(api.library.registry(), TypeId::of::<MouseButton>());
    let mut input = api.module("input");
    if let Some(key) = key {
        let checks: [(&str, Check<KeyCode>); 3] = [
            ("pressed", ButtonInput::pressed),
            ("just_pressed", ButtonInput::just_pressed),
            ("just_released", ButtonInput::just_released),
        ];
        for (name, check) in checks {
            input.add_described(name, vec![key.clone()], Ty::Bool, move |ctx, args| {
                button(ctx, args[0], check)
            });
        }
    }
    if let Some(mouse) = mouse {
        let checks: [(&str, Check<MouseButton>); 3] = [
            ("mouse_pressed", ButtonInput::pressed),
            ("mouse_just_pressed", ButtonInput::just_pressed),
            ("mouse_just_released", ButtonInput::just_released),
        ];
        for (name, check) in checks {
            input.add_described(name, vec![mouse.clone()], Ty::Bool, move |ctx, args| {
                button(ctx, args[0], check)
            });
        }
    }
    input.add(cursor);
}

/// One of `ButtonInput`'s checks for a button, like `pressed`.
type Check<T> = fn(&ButtonInput<T>, T) -> bool;

/// Checks one button of the script's choosing against its input resource.
fn button<'gc, T>(ctx: Ctx<'gc>, value: Val<'gc>, check: Check<T>) -> RtResult<Val<'gc>>
where
    T: FromReflect + Copy + Eq + Hash + Send + Sync + 'static,
{
    let button: T = Catalog::of(ctx)?
        .to_rust(ctx, value)
        .ok_or_else(|| RtErr::InvalidArgument("that isn't a button".into()))?;
    Ok(Val::Bool(resource(ctx, |input: &ButtonInput<T>| {
        check(input, button)
    })?))
}

type Windows = QueryState<&'static Window, With<PrimaryWindow>>;
type Cameras = QueryState<(&'static Camera, &'static GlobalTransform)>;

/// The window and camera queries behind `cursor`. Building a query scans every archetype, so they
/// are built once per Vm.
#[derive(Default)]
struct CursorQueries(RefCell<Option<(Windows, Cameras)>>);

/// Where the cursor is in world space, through the first camera, or null when it's off the window.
#[native]
fn cursor(ctx: Ctx) -> Result<Option<Vec2>, RtErr> {
    let cache = ctx.fixture::<CursorQueries>();
    ctx.world(|world| {
        let mut slot = cache.0.borrow_mut();
        let (windows, cameras) =
            slot.get_or_insert_with(|| (world.query_filtered(), world.query()));
        let position = windows.iter(world).next()?.cursor_position()?;
        let (camera, transform) = cameras.iter(world).next()?;
        camera.viewport_to_world_2d(transform, position).ok()
    })
}
