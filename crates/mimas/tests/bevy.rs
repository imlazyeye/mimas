use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use bevy::{
    ecs::message::{MessageCursor, Messages},
    input::{ButtonInput, keyboard::KeyCode},
    prelude::*,
    time::TimeUpdateStrategy,
};
use mimas::{
    bevy::prelude::*,
    vm::{Ctx, RtErr, conversion::MimasType},
};

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

/// Where `run` picks up reading errors. Message buffers never rotate without fixed ticks, so they
/// have to be read through a cursor.
#[derive(Resource, Default)]
struct Errors(MessageCursor<ScriptError>);

/// How many times a script's top-level code has run, which is once per compile.
#[derive(Resource, Default)]
struct Compiles(usize);

const COUNT_UP: &str = "fn update(counter: Counter) { counter.0 += 1; }";

/// A headless app with the test types. `folder` is the scripts folder under `tests/assets`, for
/// tests that load one.
fn app(folder: Option<&str>) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: "tests/assets".into(),
            ..default()
        },
        MimasPlugin {
            scripts: folder.map(String::from),
        },
    ))
    .script_message::<Ping>()
    .script_installer(|api| {
        api.module("test")
            .add_named("compiled", |ctx: Ctx<'_>| -> Result<(), RtErr> {
                ctx.world(|world| world.resource_mut::<Compiles>().0 += 1)
            });
    })
    .init_resource::<Score>()
    .init_resource::<Errors>()
    .init_resource::<Compiles>();
    app
}

/// Adds a `.mim` asset by hand, as loading one from disk would.
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

/// Runs `frames` updates and collects every `ScriptError` they produced.
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

/// Runs frames until `done`, for assets that load from disk in the background.
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

fn counter(app: &App, entity: Entity) -> i64 {
    app.world().get::<Counter>(entity).unwrap().0
}

fn score(app: &App) -> i64 {
    app.world().resource::<Score>().0
}

fn compiles(app: &App) -> usize {
    app.world().resource::<Compiles>().0
}

#[test]
fn hooks_run_per_entity_and_start_survives_reload() {
    const SOURCE: &str = "
        fn start(counter: Counter) { counter.0 = 50; }
        fn update(counter: Counter) { counter.0 += 1; }
    ";
    let (mut app, a, script) = scripted(SOURCE);
    let b = app
        .world_mut()
        .spawn((Script(script.clone()), Counter(0)))
        .id();
    let errors = run(&mut app, 4);
    assert!(errors.is_empty(), "{errors:?}");
    let value = counter(&app, a);
    assert!(value > 50, "{value}");
    assert_eq!(counter(&app, b), value);

    edit(
        &mut app,
        &script,
        &format!("{SOURCE}\nfn fixed_update() {{}}"),
    );
    run(&mut app, 2);
    assert_eq!(counter(&app, a), value + 2);
}

#[test]
fn get_writes_back_and_aliases_within_a_hook() {
    let (mut app, entity, _) = scripted(
        "
        fn update() {
            let me = script::entity();
            let first = Counter::get(me)!;
            first.0 += 1;
            let again = Counter::get(me)!;
            again.0 += 10;
        }
    ",
    );
    let errors = run(&mut app, 4);
    assert!(errors.is_empty(), "{errors:?}");
    let value = counter(&app, entity);
    assert!(value > 0 && value % 11 == 0, "{value}");
}

#[test]
fn parameters_fill_from_components_and_resources() {
    let (mut app, counted, script) = scripted(
        "
        fn update(counter: Counter?, score: Score, step = 5) {
            if let counter? = counter {
                counter.0 += step;
            } else {
                score.0 += 1;
            }
        }
    ",
    );
    // this one has no `Counter`, so its parameter is null
    app.world_mut().spawn(Script(script));
    let errors = run(&mut app, 4);
    assert!(errors.is_empty(), "{errors:?}");
    let counted = counter(&app, counted);
    assert!(counted > 0 && counted % 5 == 0, "{counted}");
    assert_eq!(score(&app), counted / 5);
}

#[test]
fn hooks_with_the_wrong_signature_fail_to_load() {
    for (source, expected) in [
        ("fn update(n: int) {}", "isn't a component or resource"),
        ("fn update() -> int { 1 }", "returns `int`"),
        ("fn stop(counter: Counter) {}", "`stop` runs after"),
    ] {
        let (mut app, _, _) = scripted(source);
        let errors = run(&mut app, 3);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains(expected), "{errors:?}");
    }
}

#[test]
fn a_missing_required_component_skips_the_hook_and_reports_once() {
    let mut app = app(None);
    let script = add(
        &mut app,
        "test.mim",
        "fn update(counter: Counter, score: Score) { score.0 += 1; }",
    );
    app.world_mut().spawn(Script(script));
    let errors = run(&mut app, 4);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("needs a `Counter`"), "{errors:?}");
    assert_eq!(score(&app), 0);
}

#[test]
fn unchanged_values_are_not_written_back() {
    let (mut app, entity, _) =
        scripted("fn update(counter: Counter) { if counter.0 > 100 { counter.0 = 0; } }");
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
fn insert_and_remove_win_over_an_earlier_get() {
    // the score only reaches 9 if the insert stuck despite the lent parameter, and the counter
    // only stays gone if the remove did
    let (mut app, entity, _) = scripted(
        "
        fn update(counter: Counter?, score: Score) {
            let me = script::entity();
            let c? = counter else return;
            if score.0 == 0 {
                Counter::insert(me, Counter(50));
                Score::insert(Score(1));
            } else if score.0 == 1 && c.0 == 50 {
                Counter::remove(me);
                Score::insert(Score(9));
            }
        }
    ",
    );
    let errors = run(&mut app, 6);
    assert!(errors.is_empty(), "{errors:?}");
    assert!(app.world().get::<Counter>(entity).is_none());
    assert_eq!(score(&app), 9);
}

#[test]
fn get_needs_a_hook() {
    let (mut app, _, _) = scripted("let score = Score::get()!; score.0 = 5;");
    let errors = run(&mut app, 3);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("hook"), "{errors:?}");
    assert_eq!(score(&app), 0);
}

#[test]
fn stop_runs_when_the_script_goes_away() {
    let (mut app, entity, _) = scripted("fn stop() { Score::insert(Score(7)); }");
    let errors = run(&mut app, 3);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(score(&app), 0);

    app.world_mut().despawn(entity);
    run(&mut app, 2);
    assert_eq!(score(&app), 7);
}

#[test]
fn entities_lists_every_holder_of_a_component() {
    let (mut app, _, _) = scripted(
        "
        fn update(score: Score) {
            for e in Counter::entities() {
                score.0 += Counter::get(e)!.0;
            }
        }
    ",
    );
    app.world_mut().spawn(Counter(2));
    app.world_mut().spawn(Counter(3));
    let errors = run(&mut app, 5);
    assert!(errors.is_empty(), "{errors:?}");
    let points = score(&app);
    assert!(points > 0 && points % 5 == 0, "{points}");
}

#[test]
fn messages_are_read_once_per_entity_and_survive_reload() {
    const SOURCE: &str =
        "fn update(score: Score) { for ping in Ping::read() { score.0 += ping.0; } }";
    let (mut app, _, script) = scripted(SOURCE);
    app.world_mut().spawn(Script(script.clone()));
    assert!(run(&mut app, 3).is_empty());

    // the buffers never rotate here, so reading twice would count the ping again
    app.world_mut().write_message(Ping(21));
    run(&mut app, 3);
    assert_eq!(score(&app), 42);
    edit(&mut app, &script, &format!("{SOURCE}\nfn stop() {{}}"));
    let errors = run(&mut app, 3);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(score(&app), 42);
}

#[test]
fn errors_report_once_and_a_fixed_reload_recovers() {
    let (mut app, entity, script) = scripted("fn update() { let x: int = \"nope\"; }");
    let errors = run(&mut app, 3);
    assert_eq!(errors.len(), 1, "{errors:?}");

    edit(
        &mut app,
        &script,
        "fn update() { let xs = [1]; print(xs[5]); }",
    );
    let errors = run(&mut app, 5);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("index out of bounds"), "{errors:?}");

    edit(&mut app, &script, COUNT_UP);
    let errors = run(&mut app, 3);
    assert!(errors.is_empty(), "{errors:?}");
    assert!(counter(&app, entity) > 0);
}

#[test]
fn keys_are_bevys_own_enum() {
    let (mut app, _, _) = scripted(
        "fn update(score: Score) { if input::just_pressed(bevy::KeyCode::Space) { score.0 += 1; } }",
    );
    app.init_resource::<ButtonInput<KeyCode>>();
    assert!(run(&mut app, 3).is_empty());
    assert_eq!(score(&app), 0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    run(&mut app, 1);
    assert_eq!(score(&app), 1);

    let (mut app, _, _) = scripted("fn update() { input::pressed(bevy::KeyCode::Spcae); }");
    let errors = run(&mut app, 3);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("Spcae"), "{errors:?}");
}

#[test]
fn bevy_types_are_reflected_into_the_bevy_module() {
    let mut app = app(None);
    let script = add(
        &mut app,
        "test.mim",
        "
        let spawned = bevy::Entity::spawn();
        bevy::Transform::insert(spawned, bevy::Transform::from_xyz(3.0, 0.0, 0.0));
        bevy::Visibility::insert(spawned, bevy::Visibility::Hidden);

        fn update(t: bevy::Transform) {
            t.translation.x += 1.0;
        }
    ",
    );
    let scripted = app
        .world_mut()
        .spawn((Script(script), Transform::default()))
        .id();
    let errors = run(&mut app, 3);
    assert!(errors.is_empty(), "{errors:?}");
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
    assert_eq!(*spawned[0].1, Visibility::Hidden);
}

#[test]
fn host_natives_reach_the_world() {
    let mut app = app(None);
    app.script_installer(|api| {
        api.module("game").add_named(
            "spawn_counter",
            |ctx: Ctx<'_>, n: i64| -> Result<mimas::bevy::Entity, RtErr> {
                ctx.world(|world| world.spawn(Counter(n)).id().into())
            },
        );
    });
    let script = add(&mut app, "test.mim", "game::spawn_counter(3);");
    app.world_mut().spawn(Script(script));
    let errors = run(&mut app, 3);
    assert!(errors.is_empty(), "{errors:?}");
    let counters = app
        .world_mut()
        .query::<&Counter>()
        .iter(app.world())
        .filter(|counter| counter.0 == 3)
        .count();
    assert_eq!(counters, 1);
}

#[test]
fn fixed_update_runs_on_the_fixed_clock() {
    let (mut app, entity, _) = scripted(
        "
        fn update(score: Score) { score.0 += 1; }
        fn fixed_update(counter: Counter) { counter.0 += 1; }
    ",
    );
    // a tenth of a second per frame is several 64 Hz fixed ticks
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    let errors = run(&mut app, 6);
    assert!(errors.is_empty(), "{errors:?}");
    let (frames, ticks) = (score(&app), counter(&app, entity));
    assert!(
        frames > 0 && ticks > 2 * frames,
        "{frames} frames, {ticks} ticks"
    );
}

#[test]
fn attach_gives_an_entity_a_script_from_disk() {
    let mut app = app(Some("scripts"));
    let script = add(
        &mut app,
        "test.mim",
        "
        fn start() {
            let other = bevy::Entity::spawn();
            Counter::insert(other, Counter(0));
            script::attach(other, \"attached.mim\");
        }
    ",
    );
    app.world_mut().spawn(Script(script));
    let errors = settle(&mut app, |app| {
        app.world()
            .iter_entities()
            .any(|e| e.get::<Counter>().is_some_and(|c| c.0 == 50))
    });
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn modules_compile_into_every_script() {
    let mut app = app(None);
    // like any asset, a module added by hand lives as long as a handle to it does
    let _steps = add(
        &mut app,
        "steps.mim",
        "
        // a comment first, to make sure the declaration is found by token
        module @;

        pub fn step(counter: Counter) { counter.0 += 1; }

        // not a hook, since only a script's own root functions are
        pub fn update() { panic(\"a module's update ran as a hook\"); }
    ",
    );
    let direct = add(
        &mut app,
        "direct.mim",
        "fn update(counter: Counter) { steps::step(counter); }",
    );
    let imported = add(
        &mut app,
        "imported.mim",
        "use steps::step; fn update(counter: Counter) { step(counter); step(counter); }",
    );
    let a = app.world_mut().spawn((Script(direct), Counter(0))).id();
    let b = app.world_mut().spawn((Script(imported), Counter(0))).id();
    let errors = run(&mut app, 4);
    assert!(errors.is_empty(), "{errors:?}");
    let ran = counter(&app, a);
    assert!(ran > 0);
    assert_eq!(counter(&app, b), 2 * ran);
}

#[test]
fn any_change_recompiles_every_script() {
    const STEPS: &str =
        "module @; pub const SIZE = 1; pub fn step(counter: Counter) { counter.0 += SIZE; }";
    const SCRIPT: &str = "test::compiled(); fn update(counter: Counter) { steps::step(counter); }";
    let mut app = app(None);
    let steps = add(&mut app, "steps.mim", STEPS);
    let a = add(&mut app, "a.mim", SCRIPT);
    let b = add(&mut app, "b.mim", SCRIPT);
    let entity = app.world_mut().spawn((Script(a.clone()), Counter(0))).id();
    app.world_mut().spawn((Script(b), Counter(0)));
    let errors = run(&mut app, 4);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(compiles(&app), 2);

    edit(&mut app, &steps, &STEPS.replace("SIZE = 1", "SIZE = 100"));
    run(&mut app, 3);
    assert_eq!(compiles(&app), 4);
    let before = counter(&app, entity);
    run(&mut app, 1);
    assert_eq!(counter(&app, entity), before + 100);

    edit(&mut app, &a, &format!("{SCRIPT}\nfn stop() {{}}"));
    run(&mut app, 3);
    assert_eq!(compiles(&app), 6);
}

#[test]
fn a_broken_module_reports_once() {
    let mut app = app(None);
    let _steps = add(
        &mut app,
        "steps.mim",
        "module @; pub fn step() { let x: int = \"nope\"; }",
    );
    let a = add(&mut app, "a.mim", "fn update() { steps::step(); }");
    let b = add(&mut app, "b.mim", "fn update() { steps::step(); }");
    app.world_mut().spawn(Script(a));
    app.world_mut().spawn(Script(b));
    let errors = run(&mut app, 4);
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn scripts_wait_for_the_folder_unless_it_fails() {
    let mut waiting = app(Some("scripts"));
    let counted = waiting
        .world()
        .resource::<AssetServer>()
        .load("scripts/counted.mim");
    let entity = waiting
        .world_mut()
        .spawn((Script(counted), Counter(0)))
        .id();
    // compiling before `steps.mim` had loaded would have reported it missing
    let errors = settle(&mut waiting, |app| counter(app, entity) > 0);
    assert!(errors.is_empty(), "{errors:?}");

    let mut failed = app(Some("nowhere"));
    let script = add(&mut failed, "test.mim", COUNT_UP);
    let entity = failed.world_mut().spawn((Script(script), Counter(0))).id();
    let errors = settle(&mut failed, |app| counter(app, entity) > 0);
    assert!(errors.is_empty(), "{errors:?}");
}

// fails to compile when Bevy's glam and the one mimas pins drift apart
#[test]
fn bevy_math_types_are_mimas_types() {
    fn mimas_type<T: for<'gc> MimasType<'gc>>() {}
    mimas_type::<Vec2>();
    mimas_type::<Vec3>();
    mimas_type::<Quat>();
}
