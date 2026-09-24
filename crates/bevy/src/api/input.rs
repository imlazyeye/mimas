use std::{any::TypeId, cell::RefCell, hash::Hash};

use bevy::{
    camera::Camera,
    ecs::query::QueryState,
    input::{ButtonInput, keyboard::KeyCode, mouse::MouseButton},
    math::Vec2,
    prelude::{GlobalTransform, With},
    reflect::{FromReflect, Typed},
    window::{PrimaryWindow, Window},
};

use crate::{plugin::ScriptCtx, reflect::Convert};
use macros::native;
use vm::{
    Ctx, RtErr, Ty, Val,
    api::{Api, ModuleApi},
};

use super::install::resource;

/// Registers `bevy::input`.
pub(crate) fn install(api: &mut Api) {
    /// Adds `pressed`, `just_pressed`, and `just_released` for one type of button.
    fn buttons<T>(input: &mut ModuleApi, ty: Ty, prefix: &str)
    where
        T: FromReflect + Typed + Copy + Eq + Hash + Send + Sync + 'static,
    {
        type Check<T> = fn(&ButtonInput<T>, T) -> bool;

        let checks: [(&str, Check<T>); 3] = [
            ("pressed", ButtonInput::pressed),
            ("just_pressed", ButtonInput::just_pressed),
            ("just_released", ButtonInput::just_released),
        ];
        for (name, check) in checks {
            input.add_described(
                format!("{prefix}{name}"),
                vec![("button".to_string(), Some(ty.clone()))],
                Ty::Bool,
                move |ctx, args| {
                    let button = Convert::to_rust(ctx, args[0])
                        .ok_or_else(|| RtErr::InvalidArgument("that isn't a button".into()))?;
                    Ok(Val::Bool(resource(ctx, |input: &ButtonInput<T>| {
                        check(input, button)
                    })?))
                },
            );
        }
    }

    let registered = "`MimasPlugin` hands the catalog the input enums";
    let registry = api.library.registry();
    let key = registry
        .ty_of_id(TypeId::of::<KeyCode>())
        .expect(registered);
    let mouse = registry
        .ty_of_id(TypeId::of::<MouseButton>())
        .expect(registered);
    let mut input = api.module("bevy::input");
    buttons::<KeyCode>(&mut input, key, "");
    buttons::<MouseButton>(&mut input, mouse, "mouse_");
    input.add(cursor);
}

type Windows = QueryState<&'static Window, With<PrimaryWindow>>;
type Cameras = QueryState<(&'static Camera, &'static GlobalTransform)>;

/// Cached queries for `cursor`.
#[derive(Default)]
struct CursorQueries(RefCell<Option<(Windows, Cameras)>>);

/// The cursor's position in world space, or null if it's outside the window.
#[native]
fn cursor(ctx: Ctx) -> Option<Vec2> {
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
