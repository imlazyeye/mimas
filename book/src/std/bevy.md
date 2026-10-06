# bevy

What the [Bevy plugin](../extension/bevy.md) adds for scripts. Bevy's own types (like `bevy::Transform`) live in this module too, but they come from your app's type registry and aren't listed here. [Components, resources, and messages](../extension/bevy.md#components-resources-and-messages) covers what scripts can call on them.

| Item | Summary |
| :-- | :-- |
| [`bevy::start`](#start) | Registers `f` to run once for each entity with the script, before its first `update` |
| [`bevy::update`](#update) | Registers `f` to run every frame |
| [`bevy::fixed_update`](#fixed_update) | Registers `f` to run every fixed tick |
| [`bevy::stop`](#stop) | Registers `f` to run when an entity's script stops |
| [`bevy::Entity::spawn`](#spawn) | Spawns an entity with no components and returns it |
| [`bevy::Entity::despawn`](#despawn) | Despawns the entity, and returns `false` if it was already gone |
| [`bevy::Entity::exists`](#exists) | Returns `true` if the entity hasn't been despawned |
| [`bevy::ecs::me`](#me) | Returns the entity the running hook is for |
| [`bevy::ecs::attach_script`](#attach_script) | Gives `entity` the script at `path` |
| [`bevy::time::delta`](#delta) | Returns the seconds since the last frame |
| [`bevy::time::elapsed`](#elapsed) | Returns the seconds since the app started |
| [`bevy::input::pressed`](#pressed) | Returns `true` while the key `button` is held down |
| [`bevy::input::just_pressed`](#just_pressed) | Returns `true` during the frame the key `button` went down |
| [`bevy::input::just_released`](#just_released) | Returns `true` during the frame the key `button` came up |
| [`bevy::input::mouse_pressed`](#mouse_pressed) | Returns `true` while the mouse button `button` is held down |
| [`bevy::input::mouse_just_pressed`](#mouse_just_pressed) | Returns `true` during the frame the mouse button `button` went down |
| [`bevy::input::mouse_just_released`](#mouse_just_released) | Returns `true` during the frame the mouse button `button` came up |
| [`bevy::input::cursor`](#cursor) | Returns the cursor's position in 2D world space, as the first camera sees it |
| [`bevy::log::info`](#info) | Logs `message` at the info level through Bevy's logger |
| [`bevy::log::warn`](#warn) | Logs `message` at the warn level through Bevy's logger |
| [`bevy::log::error`](#error) | Logs `message` at the error level through Bevy's logger |

## Hooks

Each of these registers a function for the plugin to run. `f` can be a function by name, a method, or a closure. Its parameters are filled by type on every run, each with a component of the entity or a resource, and a `T?` parameter accepts an entity that has no `T`. Trailing parameters with defaults keep them.

Call these from a script's top-level code, which runs once each time the script compiles. Registering faults from inside a hook, when `f` isn't a function, or when a parameter is neither a component nor a resource. [Hooks](../extension/bevy.md#hooks) explains how they run.

```mimas ignore
fn fall(t: bevy::Transform) {
    t.translation.y -= 9.8 * bevy::time::delta();
}

bevy::update(fall);
```

### `start`

```mimas ignore
fn start(f)
```

Registers `f` to run once for each entity with the script, before its first `update`. If it faults, or the entity is missing a component it needs, it's tried again the next frame.

### `update`

```mimas ignore
fn update(f)
```

Registers `f` to run every frame, in Bevy's `Update` schedule.

### `fixed_update`

```mimas ignore
fn fixed_update(f)
```

Registers `f` to run every fixed tick, in Bevy's `FixedUpdate` schedule.

### `stop`

```mimas ignore
fn stop(f)
```

Registers `f` to run when an entity's script stops, which is when its `Script` is removed or replaced, or when the entity despawns. A despawned entity has no components left, so `f` should only take resources.

## `bevy::Entity`

```mimas ignore
struct Entity(int)
```

An entity in the Bevy world, held as the bits of its id. Scripts get one from [`spawn`](#spawn), [`bevy::ecs::me`](#me), a component's `entities()`, or an `Entity` field of another type.

### `spawn`

```mimas ignore
fn spawn() -> Entity
```

Spawns an entity with no components and returns it.

```mimas ignore
let brick = bevy::Entity::spawn();
let placed = bevy::Transform::default();
placed.translation.x = 120.0;
bevy::Transform::insert(brick, placed);
```

### `despawn`

```mimas ignore
fn despawn(self) -> bool
```

Despawns the entity, and returns `false` if it was already gone.

### `exists`

```mimas ignore
fn exists(self) -> bool
```

Returns `true` if the entity hasn't been despawned.

## `bevy::ecs`

### `me`

```mimas ignore
fn me() -> Entity
```

Returns the entity the running hook is for. Faults outside a hook, like in a script's top-level code.

### `attach_script`

```mimas ignore
fn attach_script(entity: Entity, path: str)
```

Gives `entity` the script at `path` (relative to `assets/`), replacing any script it already had. Faults if the entity doesn't exist.

```mimas ignore
let ball = bevy::Entity::spawn();
bevy::ecs::attach_script(ball, "scripts/ball.mim");
```

## `bevy::time`

### `delta`

```mimas ignore
fn delta() -> float
```

Returns the seconds since the last frame. Inside `fixed_update`, it's the length of a fixed tick instead.

### `elapsed`

```mimas ignore
fn elapsed() -> float
```

Returns the seconds since the app started. Inside `fixed_update`, it follows the fixed clock.

## `bevy::input`

Keys and mouse buttons are Bevy's own enums, `bevy::KeyCode` and `bevy::MouseButton`.

```mimas ignore
bevy::update(|t: bevy::Transform| {
    if bevy::input::pressed(bevy::KeyCode::KeyA) {
        t.translation.x -= 300.0 * bevy::time::delta();
    }
});
```

### `pressed`

```mimas ignore
fn pressed(button: KeyCode) -> bool
```

Returns `true` while the key `button` is held down.

### `just_pressed`

```mimas ignore
fn just_pressed(button: KeyCode) -> bool
```

Returns `true` during the frame the key `button` went down.

### `just_released`

```mimas ignore
fn just_released(button: KeyCode) -> bool
```

Returns `true` during the frame the key `button` came up.

### `mouse_pressed`

```mimas ignore
fn mouse_pressed(button: MouseButton) -> bool
```

Returns `true` while the mouse button `button` is held down.

### `mouse_just_pressed`

```mimas ignore
fn mouse_just_pressed(button: MouseButton) -> bool
```

Returns `true` during the frame the mouse button `button` went down.

### `mouse_just_released`

```mimas ignore
fn mouse_just_released(button: MouseButton) -> bool
```

Returns `true` during the frame the mouse button `button` came up.

### `cursor`

```mimas ignore
fn cursor() -> Vec2?
```

Returns the cursor's position in 2D world space, as the first camera sees it. It's `null` while the cursor is outside the primary window. The result is a [`Vec2`](./math/vec2.md) from `std::math`.

## `bevy::log`

### `info`

```mimas ignore
fn info(message: str)
```

Logs `message` at the info level through Bevy's logger.

### `warn`

```mimas ignore
fn warn(message: str)
```

Logs `message` at the warn level through Bevy's logger.

### `error`

```mimas ignore
fn error(message: str)
```

Logs `message` at the error level through Bevy's logger.
