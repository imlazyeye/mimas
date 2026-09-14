# Bevy

The `bevy` feature runs mimas scripts inside a [Bevy](https://bevy.org) 0.19 app. Scripts are assets, so they hot reload with Bevy's `file_watcher` feature, and they're type-checked against your app's components and resources when they load.

The core scaffolding for this plugin is solid, but practical use will likely require further expansion of its abilities and support for bevy types/features. Please feel welcome to contribute these yourself or open an issue to request them!

## Example

<div id="breakout-section">
<div id="breakout-copy">

`examples/bevy` is a breakout game: three scripts on three entities, sharing a `Game` resource and a `GameEvent` message, plus a `controls` module they all use. 

You can run it locally with:

```sh
cargo run --manifest-path examples/bevy/Cargo.toml
```

### Controls
**A**: Move paddle left

**D**: Move paddle right

**Space**: Serve/Restart round

Space serves, A and D steer.
</div>
<div id="breakout-demo">
    <canvas id="breakout" width="480" height="640" tabindex="0" hidden></canvas>
    <button id="breakout-play">Run demo</button>
</div>
</div>
<script type="module">
    const button = document.getElementById('breakout-play');
    const canvas = document.getElementById('breakout');
    // the page scrolls on space unless the canvas claims it
    canvas.addEventListener('keydown', (event) => {
        if (event.code === 'Space') event.preventDefault();
    });
    button.addEventListener('click', async () => {
        button.textContent = 'Loading...';
        try {
            const game = await import('./breakout/breakout.js');
            canvas.hidden = false;
            button.remove();
            canvas.focus();
            await game.default();
        } catch (err) {
            // bevy's winit loop throws to break out of `main`, which isn't a failure
            if (String(err).includes('Using exceptions for control flow')) return;
            button.textContent = 'The demo only runs on the deployed book';
        }
    });
</script>

```admonish question title="Why not bevy_mod_scripting?"
[bevy_mod_scripting](https://github.com/makspll/bevy_mod_scripting) hosts Lua and Rhai in Bevy. mimas isn't a good fit for it right now, for a few reasons:

- mimas's garbage collector is single-threaded, but `bevy_mod_scripting` requires a script's context to be `Send`. A backend would need an unsafe wrapper and a guarantee that every native is thread-safe.
- `bevy_mod_scripting` gives scripts live handles into the world, so every field read or write runs through reflection. mimas's field access has no way to call a getter or setter, so it can't work that way. Our plugin copies Bevy's data into scripts and writes changes back instead, which has [its own advantages](#hooks).
- The other backends let `+`, `-`, and the other operators work on Bevy types like `Vec3` by calling functions registered on that type. mimas has no operator overloading yet, so scripts would have to call those as methods, like `a.add(b)`.

Additionally, `bevy_mod_scripting`'s API is built for dynamic languages. mimas is statically typed, so this plugin leans into that instead. A hook takes its components and resources as typed parameters, and a wrong field or type is caught when the script compiles. We'll explore that below!
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
fn move_along(v: Velocity, t: bevy::Transform) {
    t.translation.x += v.x * bevy::time::delta();
}

bevy::update(move_along);
```

`Velocity` reaches the script without being registered for it. Any component or resource that derives bevy's `Reflect` with `#[reflect(Component)]` or `#[reflect(Resource)]` is reachable as long as mimas can represent its fields. Bevy's own types live in a `bevy` module, like `bevy::Transform` and `bevy::Visibility`, and your app's types sit at the root.

## Scripts and modules

`MimasPlugin::default()` loads every `.mim` file under `assets/scripts` at startup. `MimasPlugin::folder(path)` loads another folder instead, and `MimasPlugin::new([path, ..])` loads just the scripts you name, which is what a web build needs, since a folder can't be listed over HTTP. A file that starts with a `module` declaration compiles into every other script and never runs on its own. Every other file compiles into its own Vm, shared by the entities whose `Script` points at it.

Adding, editing, or removing any file recompiles every script. A recompile that fails keeps the previous version running.

## Hooks

A script registers the functions it wants run by calling one of four natives in the `bevy` module. The call goes in the script's top-level code, which runs once each time the script compiles. Registering from inside a hook is an error.

| Register with | Runs |
| :-- | :-- |
| `bevy::start(f)` | once per entity, before its first `update` |
| `bevy::update(f)` | every frame |
| `bevy::fixed_update(f)` | every fixed tick |
| `bevy::stop(f)` | when the `Script` is removed or its entity despawns |

Any function value works. A root function by name, a method or associated function off an impl, or a closure written inline can all be used as a hook. A method's `self` is its first parameter, filled by type like any other.

```mimas
fn steer(t: bevy::Transform) { /* ... */ }

bevy::update(steer);
bevy::fixed_update(Ball::fixed_update);
bevy::start(|game: Game| game.reset());
```

A hook can be registered more than once, and its functions run in registration order. Nothing registered at a hook means nothing runs there.

Inside a hook, `bevy::ecs::me()` is the entity it runs for. Parameters are filled by type, like a Bevy system's: a component of that entity, or a resource. A `T?` parameter accepts an entity without the component, while a required component the entity lacks skips the hook. `stop` can only take resources, since the entity is gone by then.

A closure keeps whatever it captured alive for as long as the script is loaded, so a value it closed over lives from one hook call to the next.

A hook works on copies. Whatever it changes on a parameter, or on something it got from `get`, is written back when the hook returns. Field access in between is plain slot access, with no reflection. A value that comes back unchanged isn't written, so change detection only fires for real changes. A hook that faults skips the write back, though calls like `insert` that act on the world directly still take effect.

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
use mimas::{Ctx, bevy::prelude::*};

app.script_installer(|api| {
    api.module("game").add_named(
        "spawn_at",
        |ctx: Ctx, x: f32| -> mimas::bevy::Entity {
            ctx.world(|world| world.spawn(Transform::from_xyz(x, 0.0, 0.0)).id().into())
        },
    );
});
```

## Built in

```admonish todo title="A temporary home"
This list stands in for a proper reference. mimas doesn't have a pipeline for generating API reference docs yet, and once [it does](https://github.com/imlazyeye/mimas/issues/26), these natives will move there.
```

- `bevy::Entity::spawn()`, `e.despawn()`, `e.exists()`, `bevy::ecs::me()`, and `bevy::ecs::attach_script(e, path)`
- `T::default()` for any reflected type with `#[reflect(Default)]`, like `bevy::Transform::default()`
- `std::math::{Vec2, Vec3, Quat}`, which are Bevy's own math types
- `bevy::time::delta()` and `bevy::time::elapsed()`, which follow the fixed clock inside `fixed_update`
- `bevy::input::pressed`, `bevy::input::just_pressed`, and `bevy::input::just_released` with a `bevy::KeyCode`, their `mouse_` versions with a `bevy::MouseButton`, and `bevy::input::cursor()` in world space
- `bevy::log::info`, `bevy::log::warn`, and `bevy::log::error`

Compile errors and runtime faults print to stderr as full diagnostics, log as one line, and arrive as `ScriptError` messages. Each registered function reports its first fault and none after it, however many entities or frames it faults on, until its script reloads.

## Known limits

- Scripts can use numbers, strings, `bool`, `Vec2`, `Vec3`, `Quat`, `Entity`, `Option` and `Vec` of those, and structs and enums built from them. A type with any other field, like a `Handle<T>`, map, array, tuple, or `Duration`, is left out, and scripts can't name it. [Open an issue](https://github.com/imlazyeye/mimas/issues/new) if you need one!
- Types that contain themselves, directly or through each other, are skipped.
- A `u64` or `usize` above `i64::MAX` reaches scripts as a negative `int` with the same bits, and writes back unchanged.
- An entity runs one script at a time. Giving it another `Script` swaps the old one out, running its `stop` hooks first.
- A script can replace a resource, but not create one.
- Values cross between Bevy and scripts through reflection, which costs more than a hand-written conversion.
- mimas pins the same glam minor version as Bevy, so a host that depends on glam directly has to match it.
- Scripts run one hook at a time on the main thread, with no step limit, so a stuck loop stalls the frame.
