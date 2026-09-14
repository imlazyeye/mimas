# Bevy

The `bevy` feature runs mimas scripts inside a [Bevy](https://bevy.org) 0.19 app. Scripts are assets, so they hot reload with Bevy's `file_watcher` feature, and they're type-checked against your app's components and resources when they load.

```admonish question title="Why not bevy_mod_scripting?"
[bevy_mod_scripting](https://github.com/makspll/bevy_mod_scripting) hosts Lua, Rhai, and Rune in Bevy. Its design suits dynamic languages: script values are references into Bevy's reflection, functions are looked up at runtime, and a wrong field or argument surfaces when that line runs. mimas is statically typed. A script is checked against every type and function it can reach before it runs, and its values live in mimas's own garbage-collected heap. Plugging mimas in as another backend there would mean giving up that checking, or rebuilding what this plugin does on top of an interface built for runtime lookups.
```

## Setup

```toml
[dependencies]
bevy = "0.19"
mimas = { version = "0.1", features = ["bevy"] }
```

```rust
use bevy::prelude::*;
use mimas::bevy::prelude::*;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct Velocity {
    x: f32,
    y: f32,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MimasPlugin::default()))
        .add_systems(Startup, |mut commands: Commands, assets: Res<AssetServer>| {
            commands.spawn((
                Velocity { x: 50.0, y: 0.0 },
                Transform::default(),
                Script(assets.load("scripts/mover.mim")),
            ));
        })
        .run();
}
```

```mimas
// assets/scripts/mover.mim
fn update(v: Velocity, t: bevy::Transform) {
    t.translation.x += v.x * time::delta();
}
```

`Velocity` reaches the script without being registered for it. Any component or resource that derives `Reflect` with `#[reflect(Component)]` or `#[reflect(Resource)]` does, as long as mimas can represent its fields. Bevy's own types live in a `bevy` module, like `bevy::Transform` and `bevy::Visibility`, and your app's types sit at the root.

## Scripts and modules

`MimasPlugin::default()` loads every `.mim` file under `assets/scripts` at startup. Set `scripts` to another folder, or to `None` to load nothing up front. A file that starts with a `module` declaration compiles into every other script and never runs on its own. Every other file compiles into its own Vm, shared by the entities whose `Script` points at it.

Adding, editing, or removing any file recompiles every script. A recompile that fails keeps the previous version running.

## Hooks

A script's hooks are top-level functions, and all of them are optional.

| Hook | Runs |
| :-- | :-- |
| `start` | once per entity, before its first `update` |
| `update` | every frame |
| `fixed_update` | every fixed tick |
| `stop` | when the `Script` is removed or its entity despawns |

Inside a hook, `script::entity()` is the entity it runs for. Parameters are filled by type, like a Bevy system's: a component of that entity, or a resource. A `T?` parameter accepts an entity without the component, while a required component the entity lacks skips the hook. `stop` can only take resources, since the entity is gone by then.

A hook works on copies. Whatever it changes on a parameter, or on something it got from `get`, is written back when the hook returns. A value that comes back unchanged isn't written, so change detection only fires for real changes.

## Components, resources, and messages

| Kind | In scripts |
| :-- | :-- |
| component `T` | `T::get(e) -> T?`, `T::insert(e, value)`, `T::remove(e) -> T?`, `T::entities() -> [bevy::Entity]` |
| resource `T` | `T::get() -> T?`, `T::insert(value)` |
| message `T` | `T::read() -> [T]`, `T::write(value)` |

Messages are the one thing you register, with `app.script_message::<T>()`, because Bevy can only read them through their concrete type. Each scripted entity reads every message once.

A second `get` in the same hook returns the same copy, and `insert` or `remove` wins over an earlier `get`. `get` only works inside a hook, since its copy is written back when the hook returns.

Natives of your own go in through `app.script_installer`, and reach the world through `ScriptCtx`:

```rust
use mimas::{Ctx, bevy::prelude::*, vm::RtErr};

app.script_installer(|api| {
    api.module("game").add_named(
        "spawn_at",
        |ctx: Ctx, x: f32| -> Result<mimas::bevy::Entity, RtErr> {
            ctx.world(|world| world.spawn(Transform::from_xyz(x, 0.0, 0.0)).id().into())
        },
    );
});
```

## Built in

- `bevy::Entity::spawn()`, `e.despawn()`, `e.exists()`, `script::entity()`, and `script::attach(e, path)`
- `bevy::Transform::from_xyz(x, y, z)`
- `std::math::{Vec2, Vec3, Quat}`, which are Bevy's own math types
- `time::delta()` and `time::elapsed()`, which follow the fixed clock inside `fixed_update`
- `input::pressed`, `input::just_pressed`, and `input::just_released` with a `bevy::KeyCode`, their `mouse_` versions with a `bevy::MouseButton`, and `input::cursor()` in world space
- `log::info`, `log::warn`, and `log::error`

Compile errors and runtime faults print to stderr as full diagnostics, log as one line, and arrive as `ScriptError` messages. A hook that faults every frame reports once.

## Limits

- A field mimas can't represent keeps its whole type out of scripts. Numbers, strings, `bool`, `Vec`, `Option`, `Entity`, the glam types, and other representable types work, while handles and maps don't.
- Types that contain themselves, directly or through each other, are skipped.
- A script can replace a resource, but not create one.
- Values cross between Bevy and scripts through reflection, which costs more than a hand-written conversion.
- mimas pins the same glam minor version as Bevy, so a host that depends on glam directly has to match it.
- Scripts run one hook at a time on the main thread, with no step limit, so a stuck loop stalls the frame.

## Example

`examples/bevy` is a breakout game: three scripts on three entities, sharing a `Game` resource and a `GameEvent` message, plus a `controls` module they all use.

```sh
cargo run --manifest-path examples/bevy/Cargo.toml
```
