use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use bevy::{
    camera::RenderTargetInfo,
    ecs::message::{MessageCursor, Messages},
    input::{ButtonInput, InputPlugin, keyboard::KeyCode},
    math::DVec2,
    prelude::*,
    time::TimeUpdateStrategy,
    window::PrimaryWindow,
};
use mimas_bevy::prelude::*;
use vm::Ctx;

/// A component scripts count with.
#[derive(Component, Reflect, PartialEq)]
#[reflect(Component)]
struct Counter(i64);

/// A resource scripts add to.
#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct Score(i64);

/// A message scripts read.
#[derive(Message, Reflect, Clone)]
struct Ping(i64);

/// A component whose fields are options, lists, and an enum with data.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Bag {
    held: Option<i64>,
    items: Vec<i64>,
    mood: Mood,
}

/// Scripts can't use this one (maps don't convert).
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Ledger {
    entries: std::collections::HashMap<String, i64>,
}

/// Holds a `u64`, which can go past `i64::MAX`.
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Mask(u64);

/// A variant of each kind.
#[derive(Reflect, Default, PartialEq, Debug)]
enum Mood {
    /// Nothing's wrong.
    #[default]
    Calm,
    Hurt(i64),
    Moving {
        speed: f32,
    },
}

/// Tracks which `ScriptError`s `run` has already read.
#[derive(Resource, Default)]
struct Errors(MessageCursor<ScriptError>);

/// Counts compiles (bumped from top-level script code).
#[derive(Resource, Default)]
struct Compiles(usize);

const COUNT_UP: &str = "bevy::update(|counter: Counter| { counter.0 += 1; });";

/// A headless app loading `folder` under `tests/assets`, or no scripts at all.
fn app(folder: Option<&str>) -> App {
    match folder {
        Some(folder) => with(MimasPlugin::folder(folder)),
        None => with(MimasPlugin::new(Vec::<String>::new())),
    }
}

/// A headless app with the test types, set up with `plugin`.
fn with(plugin: MimasPlugin) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: "tests/assets".into(),
            ..default()
        },
        plugin,
    ))
    .script_message::<Ping>()
    .script_installer(|api| {
        api.module("test").add_named("compiled", |ctx: Ctx<'_>| {
            ctx.world(|world| world.resource_mut::<Compiles>().0 += 1)
        });
    })
    .init_resource::<Score>()
    .init_resource::<Errors>()
    .init_resource::<Compiles>();
    app
}

/// Adds a `.mim` asset directly instead of loading it from disk.
fn add(app: &mut App, path: &str, source: &str) -> Handle<MimasScript> {
    app.world_mut()
        .resource_mut::<Assets<MimasScript>>()
        .add(MimasScript {
            path: path.into(),
            source: source.into(),
        })
}

/// An app running `source` on one entity with a `Counter` at zero.
fn scripted(source: &str) -> (App, Entity, Handle<MimasScript>) {
    let mut app = app(None);
    let script = add(&mut app, "test.mim", source);
    let entity = app
        .world_mut()
        .spawn((Script(script.clone()), Counter(0)))
        .id();
    (app, entity, script)
}

fn edit(app: &mut App, handle: &Handle<MimasScript>, source: &str) {
    app.world_mut()
        .resource_mut::<Assets<MimasScript>>()
        .get_mut(handle)
        .unwrap()
        .source = source.into();
}

/// Runs `frames` updates and returns any errors they sent.
fn run(app: &mut App, frames: usize) -> Vec<String> {
    let mut errors = Vec::new();
    for _ in 0..frames {
        app.update();
        errors.extend(
            app.world_mut()
                .resource_scope(|world, mut seen: Mut<Errors>| {
                    seen.0
                        .read(world.resource::<Messages<ScriptError>>())
                        .map(|err| err.message.clone())
                        .collect::<Vec<_>>()
                }),
        );
    }
    errors
}

/// Runs frames until `done` (for tests waiting on assets from disk).
fn settle(app: &mut App, done: impl Fn(&App) -> bool) -> Vec<String> {
    let mut errors = Vec::new();
    let started = Instant::now();
    while !done(app) {
        assert!(started.elapsed() < Duration::from_secs(10), "{errors:?}");
        errors.extend(run(app, 1));
        sleep(Duration::from_millis(5));
    }
    errors
}

/// Runs `frames` updates and fails if a script reported anything.
fn run_ok(app: &mut App, frames: usize) {
    let errors = run(app, frames);
    assert!(errors.is_empty(), "{errors:?}");
}

/// Runs frames until `done` and fails if a script reported anything.
fn settle_ok(app: &mut App, done: impl Fn(&App) -> bool) {
    let errors = settle(app, done);
    assert!(errors.is_empty(), "{errors:?}");
}

/// Runs `frames` updates and fails unless exactly one error mentioning `expected` came out.
fn run_error(app: &mut App, frames: usize, expected: &str) {
    let errors = run(app, frames);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains(expected), "{errors:?}");
}

/// A script that has to report one error mentioning the given text.
macro_rules! test_error {
    ($(#[$attr:meta])* $name:ident, $source:expr => $expected:expr $(,)?) => {
        #[test]
        $(#[$attr])*
        fn $name() {
            let (mut app, _, _script) = scripted($source);
            run_error(&mut app, 3, $expected);
        }
    };
}

fn counter(app: &App, entity: Entity) -> i64 {
    app.world().get::<Counter>(entity).unwrap().0
}

fn score(app: &App) -> i64 {
    app.world().resource::<Score>().0
}

fn bag(app: &App, entity: Entity) -> &Bag {
    app.world().get::<Bag>(entity).unwrap()
}

fn compiles(app: &App) -> usize {
    app.world().resource::<Compiles>().0
}

#[test]
fn hooks_run_per_entity_and_survive_reload() {
    const SOURCE: &str = "
        fn begin(counter: Counter) { counter.0 = 50; }
        fn step(counter: Counter) { counter.0 += 1; }
        bevy::start(begin);
        bevy::update(step);
    ";
    let (mut app, a, script) = scripted(SOURCE);
    let b = app
        .world_mut()
        .spawn((Script(script.clone()), Counter(0)))
        .id();
    run_ok(&mut app, 4);
    let value = counter(&app, a);
    assert!(value > 50, "{value}");
    assert_eq!(counter(&app, b), value);

    edit(
        &mut app,
        &script,
        &format!("{SOURCE}\nbevy::fixed_update(|| {{}});"),
    );
    run(&mut app, 2);
    assert_eq!(counter(&app, a), value + 2);
}

#[test]
fn registered_closure_keeps_its_captures() {
    let (mut app, entity, _) = scripted(
        "
        let step = 7;
        bevy::update(|counter: Counter| { counter.0 += step; });
    ",
    );
    run_ok(&mut app, 4);
    let counted = counter(&app, entity);
    assert!(counted > 0 && counted % 7 == 0, "{counted}");
}

#[test]
fn hook_runs_registrations_in_order() {
    let (mut app, entity, _) = scripted(
        "
        bevy::update(|counter: Counter| { counter.0 = 1; });
        bevy::update(|counter: Counter| { counter.0 += 1; });
    ",
    );
    run_ok(&mut app, 4);
    // running them in the other order would always leave 1
    assert_eq!(counter(&app, entity), 2);
}

#[test]
fn get_aliases_and_writes_back() {
    let (mut app, entity, _) = scripted(
        "
        bevy::update(|| {
            let me = bevy::ecs::me();
            let first = Counter::get(me)!;
            first.0 += 1;
            let again = Counter::get(me)!;
            again.0 += 10;
        });
    ",
    );
    run_ok(&mut app, 4);
    let value = counter(&app, entity);
    assert!(value > 0 && value % 11 == 0, "{value}");
}

#[test]
fn params_fill_from_components_and_resources() {
    let (mut app, counted, script) = scripted(
        "
        fn tick(counter: Counter?, score: Score, step = 5) {
            if let counter? = counter {
                counter.0 += step;
            } else {
                score.0 += 1;
            }
        }
        bevy::update(tick);
    ",
    );
    // this one has no `Counter`, so its parameter is null
    app.world_mut().spawn(Script(script));
    run_ok(&mut app, 4);
    let counted = counter(&app, counted);
    assert!(counted > 0 && counted % 5 == 0, "{counted}");
    assert_eq!(score(&app), counted / 5);
}

test_error!(
    hook_param_not_a_component,
    "bevy::update(|n: int| {});" => "isn't a component or resource",
);

test_error!(
    hook_returns_a_value,
    "fn one() -> int { 1 } bevy::update(one);" => "expected unit",
);

test_error!(hook_is_not_a_function, "bevy::update(4);" => "takes a function");

test_error!(
    register_inside_a_hook,
    "bevy::update(|| { bevy::update(|| {}); });" => "top-level code",
);

#[test]
fn missing_component_skips_the_hook() {
    let mut app = app(None);
    let script = add(
        &mut app,
        "test.mim",
        "bevy::update(|counter: Counter, score: Score| { score.0 += 1; });",
    );
    app.world_mut().spawn(Script(script));
    run_error(&mut app, 4, "needs a `Counter`");
    assert_eq!(score(&app), 0);
}

#[test]
fn unchanged_value_is_not_written_back() {
    let (mut app, entity, _) =
        scripted("bevy::update(|counter: Counter| { if counter.0 > 100 { counter.0 = 0; } });");
    let changed = |app: &App| {
        app.world()
            .entity(entity)
            .get_ref::<Counter>()
            .unwrap()
            .last_changed()
    };
    run(&mut app, 3);
    let before = changed(&app);
    run(&mut app, 3);
    assert_eq!(before, changed(&app));

    app.world_mut().get_mut::<Counter>(entity).unwrap().0 = 200;
    let touched = changed(&app);
    run(&mut app, 1);
    assert_ne!(touched, changed(&app));
    assert_eq!(counter(&app, entity), 0);
}

#[test]
fn insert_and_remove_win_over_get() {
    // score only gets to 9 if the insert wins over the parameter, and the counter only stays gone
    // if the remove does
    let (mut app, entity, _) = scripted(
        "
        fn tick(counter: Counter?, score: Score) {
            let me = bevy::ecs::me();
            let c? = counter else return;
            if score.0 == 0 {
                Counter::insert(me, Counter(50));
                Score::insert(Score(1));
            } else if score.0 == 1 && c.0 == 50 {
                Counter::remove(me);
                Score::insert(Score(9));
            }
        }
        bevy::update(tick);
    ",
    );
    run_ok(&mut app, 6);
    assert!(app.world().get::<Counter>(entity).is_none());
    assert_eq!(score(&app), 9);
}

#[test]
fn get_outside_a_hook_fails() {
    let (mut app, _, _) = scripted("let score = Score::get()!; score.0 = 5;");
    run_error(&mut app, 3, "hook");
    assert_eq!(score(&app), 0);
}

#[test]
fn stop_runs_on_despawn() {
    let (mut app, entity, _) = scripted("bevy::stop(|| { Score::insert(Score(7)); });");
    run_ok(&mut app, 3);
    assert_eq!(score(&app), 0);

    app.world_mut().despawn(entity);
    run(&mut app, 2);
    assert_eq!(score(&app), 7);
}

#[test]
fn entities_lists_every_holder() {
    let (mut app, _, _) = scripted(
        "
        bevy::update(|score: Score| {
            for e in Counter::entities() {
                score.0 += Counter::get(e)!.0;
            }
        });
    ",
    );
    app.world_mut().spawn(Counter(2));
    app.world_mut().spawn(Counter(3));
    run_ok(&mut app, 5);
    let points = score(&app);
    assert!(points > 0 && points % 5 == 0, "{points}");
}

#[test]
fn messages_read_once_per_entity() {
    const SOURCE: &str =
        "bevy::update(|score: Score| { for ping in Ping::read() { score.0 += ping.0; } });";
    let (mut app, _, script) = scripted(SOURCE);
    app.world_mut().spawn(Script(script.clone()));
    assert!(run(&mut app, 3).is_empty());

    // buffers don't rotate in this app, so a double read would count the ping twice
    app.world_mut().write_message(Ping(21));
    run(&mut app, 3);
    assert_eq!(score(&app), 42);
    edit(
        &mut app,
        &script,
        &format!("{SOURCE}\nbevy::stop(|| {{}});"),
    );
    run_ok(&mut app, 3);
    assert_eq!(score(&app), 42);
}

#[test]
fn fault_reports_once_and_reload_recovers() {
    let (mut app, entity, script) = scripted("bevy::update(|| { let x: int = \"nope\"; });");
    run_error(&mut app, 3, "mismatched types");

    edit(
        &mut app,
        &script,
        "bevy::update(|| { let xs = [1]; print(xs[5]); });",
    );
    run_error(&mut app, 5, "index out of bounds");

    edit(&mut app, &script, COUNT_UP);
    run_ok(&mut app, 3);
    assert!(counter(&app, entity) > 0);
}

#[test]
fn input_takes_bevys_key_enum() {
    let (mut app, _, _) = scripted(
        "bevy::update(|score: Score| { if bevy::input::just_pressed(bevy::KeyCode::Space) { score.0 += 1; } });",
    );
    app.init_resource::<ButtonInput<KeyCode>>();
    assert!(run(&mut app, 3).is_empty());
    assert_eq!(score(&app), 0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    run(&mut app, 1);
    assert_eq!(score(&app), 1);

    let (mut app, _, _) =
        scripted("bevy::update(|| { bevy::input::pressed(bevy::KeyCode::Spcae); });");
    run_error(&mut app, 3, "Spcae");
}

#[test]
fn bevy_types_land_in_the_bevy_module() {
    let mut app = app(None);
    let script = add(
        &mut app,
        "test.mim",
        "
        let spawned = bevy::Entity::spawn();
        let placed = bevy::Transform::default();
        placed.translation.x = 3.0;
        bevy::Transform::insert(spawned, placed);
        bevy::Visibility::insert(spawned, bevy::Visibility::Hidden);

        bevy::update(|t: bevy::Transform| {
            t.translation.x += 1.0;
        });
    ",
    );
    let scripted = app
        .world_mut()
        .spawn((Script(script), Transform::default()))
        .id();
    run_ok(&mut app, 3);
    let moved = app
        .world()
        .get::<Transform>(scripted)
        .unwrap()
        .translation
        .x;
    assert!(moved > 0.0, "{moved}");
    let spawned: Vec<(&Transform, &Visibility)> = app
        .world_mut()
        .query_filtered::<(&Transform, &Visibility), Without<Script>>()
        .iter(app.world())
        .collect();
    assert_eq!(spawned.len(), 1);
    assert_eq!(spawned[0].0.translation.x, 3.0);
    assert_eq!(spawned[0].0.scale, Vec3::ONE);
    assert_eq!(*spawned[0].1, Visibility::Hidden);
}

#[test]
fn host_native_reaches_the_world() {
    let mut app = app(None);
    app.script_installer(|api| {
        api.module("game").add_named(
            "spawn_counter",
            |ctx: Ctx<'_>, n: i64| -> mimas_bevy::Entity {
                ctx.world(|world| world.spawn(Counter(n)).id().into())
            },
        );
    });
    let script = add(&mut app, "test.mim", "game::spawn_counter(3);");
    app.world_mut().spawn(Script(script));
    run_ok(&mut app, 3);
    let counters = app
        .world_mut()
        .query::<&Counter>()
        .iter(app.world())
        .filter(|counter| counter.0 == 3)
        .count();
    assert_eq!(counters, 1);
}

#[test]
fn fixed_update_follows_the_fixed_clock() {
    let (mut app, entity, _) = scripted(
        "
        bevy::update(|score: Score| { score.0 += 1; });
        bevy::fixed_update(|counter: Counter| { counter.0 += 1; });
    ",
    );
    // a tenth of a second per frame is several 64 Hz fixed ticks
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    run_ok(&mut app, 6);
    let (frames, ticks) = (score(&app), counter(&app, entity));
    assert!(
        frames > 0 && ticks > 2 * frames,
        "{frames} frames, {ticks} ticks"
    );
}

#[test]
fn attach_script_loads_from_disk() {
    let mut app = app(Some("scripts"));
    let script = add(
        &mut app,
        "test.mim",
        "
        bevy::start(|| {
            let other = bevy::Entity::spawn();
            Counter::insert(other, Counter(0));
            bevy::ecs::attach_script(other, \"attached.mim\");
        });
    ",
    );
    app.world_mut().spawn(Script(script));
    settle_ok(&mut app, |app| {
        app.world()
            .iter_entities()
            .any(|e| e.get::<Counter>().is_some_and(|c| c.0 == 50))
    });
}

#[test]
fn module_compiles_into_every_script() {
    let mut app = app(None);
    // assets only stick around while something holds their handle
    let _steps = add(
        &mut app,
        "steps.mim",
        "
        // a comment before `module` shouldn't matter
        module @;

        pub fn step(counter: Counter) { counter.0 += 1; }

        // nothing registers this, so it never runs
        pub fn unused() { panic(\"a module fn ran on its own\"); }
    ",
    );
    let direct = add(
        &mut app,
        "direct.mim",
        "bevy::update(|counter: Counter| { steps::step(counter); });",
    );
    let imported = add(
        &mut app,
        "imported.mim",
        "use steps::step; bevy::update(|counter: Counter| { step(counter); step(counter); });",
    );
    let a = app.world_mut().spawn((Script(direct), Counter(0))).id();
    let b = app.world_mut().spawn((Script(imported), Counter(0))).id();
    run_ok(&mut app, 4);
    let ran = counter(&app, a);
    assert!(ran > 0);
    assert_eq!(counter(&app, b), 2 * ran);
}

#[test]
fn any_change_recompiles_every_script() {
    const STEPS: &str =
        "module @; pub const SIZE = 1; pub fn step(counter: Counter) { counter.0 += SIZE; }";
    const SCRIPT: &str =
        "test::compiled(); bevy::update(|counter: Counter| { steps::step(counter); });";
    let mut app = app(None);
    let steps = add(&mut app, "steps.mim", STEPS);
    let a = add(&mut app, "a.mim", SCRIPT);
    let b = add(&mut app, "b.mim", SCRIPT);
    let entity = app.world_mut().spawn((Script(a.clone()), Counter(0))).id();
    app.world_mut().spawn((Script(b), Counter(0)));
    run_ok(&mut app, 4);
    assert_eq!(compiles(&app), 2);

    edit(&mut app, &steps, &STEPS.replace("SIZE = 1", "SIZE = 100"));
    run(&mut app, 3);
    assert_eq!(compiles(&app), 4);
    let before = counter(&app, entity);
    run(&mut app, 1);
    assert_eq!(counter(&app, entity), before + 100);

    edit(&mut app, &a, &format!("{SCRIPT}\nbevy::stop(|| {{}});"));
    run(&mut app, 3);
    assert_eq!(compiles(&app), 6);
}

#[test]
fn broken_module_reports_once() {
    let mut app = app(None);
    let _steps = add(
        &mut app,
        "steps.mim",
        "module @; pub fn step() { let x: int = \"nope\"; }",
    );
    let a = add(&mut app, "a.mim", "bevy::update(|| { steps::step(); });");
    let b = add(&mut app, "b.mim", "bevy::update(|| { steps::step(); });");
    app.world_mut().spawn(Script(a));
    app.world_mut().spawn(Script(b));
    run_error(&mut app, 4, "mismatched types");
}

#[test]
fn compile_waits_for_the_folder() {
    let mut waiting = app(Some("scripts"));
    let counted = waiting
        .world()
        .resource::<AssetServer>()
        .load("scripts/counted.mim");
    let entity = waiting
        .world_mut()
        .spawn((Script(counted), Counter(0)))
        .id();
    // if this compiled before `steps.mim` loaded, it'd error
    settle_ok(&mut waiting, |app| counter(app, entity) > 0);

    let mut failed = app(Some("nowhere"));
    let script = add(&mut failed, "test.mim", COUNT_UP);
    let entity = failed.world_mut().spawn((Script(script), Counter(0))).id();
    settle_ok(&mut failed, |app| counter(app, entity) > 0);
}

#[test]
fn named_scripts_load_without_a_folder() {
    let mut app = with(MimasPlugin::new([
        "scripts/counted.mim",
        "scripts/steps.mim",
    ]));
    let counted = app
        .world()
        .resource::<AssetServer>()
        .load("scripts/counted.mim");
    let entity = app.world_mut().spawn((Script(counted), Counter(0))).id();
    settle_ok(&mut app, |app| counter(app, entity) > 0);
}

#[test]
fn options_lists_and_data_variants_round_trip() {
    let mut app = app(None);
    let script = add(
        &mut app,
        "test.mim",
        "
        bevy::update(|bag: Bag| {
            match bag.mood {
                Mood::Calm => {
                    bag.mood = Mood::Hurt(3);
                    bag.held = 7;
                    bag.items.push(1);
                }
                Mood::Hurt(n) => {
                    bag.mood = Mood::Moving { speed = n.to_float() };
                    bag.held = null;
                    bag.items = [4, 5, 6];
                }
                Mood::Moving { speed } => {
                    bag.mood = Mood::Moving { speed = speed + 1.0 };
                    bag.items[0] += 10;
                }
            }
        });
    ",
    );
    let entity = app.world_mut().spawn((Script(script), Bag::default())).id();
    settle_ok(&mut app, |app| bag(app, entity).mood != Mood::Calm);
    assert_eq!(bag(&app, entity).mood, Mood::Hurt(3));
    assert_eq!(bag(&app, entity).held, Some(7));
    assert_eq!(bag(&app, entity).items, [1]);

    run_ok(&mut app, 1);
    assert_eq!(bag(&app, entity).mood, Mood::Moving { speed: 3.0 });
    assert_eq!(bag(&app, entity).held, None);
    assert_eq!(bag(&app, entity).items, [4, 5, 6]);

    run(&mut app, 1);
    assert_eq!(bag(&app, entity).mood, Mood::Moving { speed: 4.0 });
    assert_eq!(bag(&app, entity).items, [14, 5, 6]);
}

#[test]
fn shortened_list_is_written_back() {
    let mut app = app(None);
    let script = add(
        &mut app,
        "test.mim",
        "bevy::update(|bag: Bag| { bag.items = [1]; });",
    );
    let bag_of_three = Bag {
        items: vec![4, 5, 6],
        ..default()
    };
    let entity = app.world_mut().spawn((Script(script), bag_of_three)).id();
    run_ok(&mut app, 3);
    assert_eq!(bag(&app, entity).items, [1]);
}

#[test]
fn u64_above_i64_max_keeps_its_bits() {
    let (mut app, entity, _) = scripted("bevy::update(|mask: Mask| { mask.0 -= 1; });");
    app.world_mut().entity_mut(entity).insert(Mask(u64::MAX));
    run_ok(&mut app, 3);
    let mask = app.world().get::<Mask>(entity).unwrap().0;
    assert!((u64::MAX - 3..u64::MAX).contains(&mask), "{mask}");
}

#[test]
fn swapped_script_stops_then_starts() {
    let mut app = app(None);
    let old = add(
        &mut app,
        "old.mim",
        "
        bevy::update(|counter: Counter| { counter.0 += 1; });
        bevy::stop(|score: Score| { score.0 += 100; });
    ",
    );
    let new = add(
        &mut app,
        "new.mim",
        "
        bevy::start(|score: Score| { score.0 += 1; });
        bevy::update(|counter: Counter| { counter.0 -= 1; });
    ",
    );
    let entity = app.world_mut().spawn((Script(old), Counter(0))).id();
    run_ok(&mut app, 3);
    let counted = counter(&app, entity);
    assert!(counted > 0, "{counted}");
    assert_eq!(score(&app), 0);

    app.world_mut().entity_mut(entity).insert(Script(new));
    run_ok(&mut app, 3);
    assert_eq!(score(&app), 101);
    assert!(counter(&app, entity) < counted);
}

#[test]
fn start_that_despawns_goes_to_stop() {
    // hold the handle so the script stays loaded after its entity despawns
    let (mut app, entity, _script) = scripted(
        "
        bevy::start(|| { bevy::ecs::me().despawn(); });
        bevy::update(|score: Score| { score.0 += 1; });
        bevy::stop(|score: Score| { score.0 += 100; });
    ",
    );
    run_ok(&mut app, 4);
    assert!(app.world().get_entity(entity).is_err());
    assert_eq!(score(&app), 100);
}

#[test]
fn start_retries_until_it_runs() {
    let mut app = app(None);
    let script = add(
        &mut app,
        "test.mim",
        "
        bevy::start(|counter: Counter| { counter.0 = 50; });
        bevy::update(|counter: Counter| { counter.0 += 1; });
    ",
    );
    let entity = app.world_mut().spawn(Script(script)).id();
    run_error(&mut app, 3, "needs a `Counter`");

    app.world_mut().entity_mut(entity).insert(Counter(0));
    run_ok(&mut app, 3);
    assert!(counter(&app, entity) > 50);
}

#[test]
fn cursor_maps_through_the_camera() {
    let (mut app, _, _) = scripted(
        "
        bevy::update(|score: Score| {
            score.0 = if let at? = bevy::input::cursor() {
                if (at.x - 50.0).abs() < 0.01 && (at.y - 25.0).abs() < 0.01 { 1 } else { 2 }
            } else {
                3
            };
        });
    ",
    );
    let mut camera = Camera::default();
    camera.computed.target_info = Some(RenderTargetInfo {
        physical_size: UVec2::new(200, 100),
        scale_factor: 1.0,
    });
    camera.computed.clip_from_view =
        Mat4::orthographic_rh(-100.0, 100.0, -50.0, 50.0, -1000.0, 1000.0);
    app.world_mut().spawn((camera, GlobalTransform::IDENTITY));
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    run_ok(&mut app, 3);
    assert_eq!(score(&app), 3);

    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_physical_cursor_position(Some(DVec2::new(150.0, 25.0)));
    run(&mut app, 1);
    assert_eq!(score(&app), 1);
}

test_error!(
    unholdable_field_leaves_the_type_out,
    "bevy::update(|ledger: Ledger| {});" => "undefined variable",
);

#[test]
fn bevy_crates_cover_the_registry() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        InputPlugin,
        MimasPlugin::new(Vec::<String>::new()),
    ));
    let registry = app.world().resource::<AppTypeRegistry>().read();
    let mut reached: Vec<&str> = registry
        .iter()
        .filter_map(|registration| registration.type_info().type_path_table().crate_name())
        .filter(|name| name.starts_with("bevy_"))
        .collect();
    reached.sort();
    reached.dedup();
    assert!(reached.contains(&"bevy_transform"), "{reached:?}");
    let missing: Vec<&str> = reached
        .into_iter()
        .filter(|name| !mimas_bevy::BEVY_CRATES.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "add these to `BEVY_CRATES`: {missing:?}"
    );
}

#[test]
fn reflected_docs_reach_the_library() {
    static DOCS: std::sync::Mutex<Vec<(String, String)>> = std::sync::Mutex::new(Vec::new());
    let (mut app, _, _script) = scripted(COUNT_UP);
    app.script_installer(|api| {
        let mut docs = DOCS.lock().unwrap();
        for adt in api.library.adts() {
            docs.push((adt.name.clone(), adt.doc.clone()));
            for variant in &adt.variants {
                let name = format!("{}::{}", adt.name, variant.name);
                docs.push((name, variant.doc.clone()));
            }
        }
    });
    run_ok(&mut app, 3);

    let docs = DOCS.lock().unwrap();
    let doc = |name: &str| {
        docs.iter()
            .find(|(n, _)| n == name)
            .map(|(_, d)| d.as_str())
    };
    assert_eq!(doc("Counter"), Some(" A component scripts count with."));
    assert_eq!(doc("Mood::Calm"), Some(" Nothing's wrong."));
    assert_eq!(doc("Mood::Hurt"), Some(""));
    assert!(
        doc("Transform")
            .unwrap()
            .starts_with(" Describe the position of an entity")
    );
}
