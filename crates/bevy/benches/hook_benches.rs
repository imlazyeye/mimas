use std::time::Duration;

use bevy::{ecs::message::Messages, prelude::*};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mimas_bevy::prelude::*;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct Velocity {
    x: f32,
    y: f32,
}

const ENTITIES: [usize; 3] = [100, 1_000, 10_000];

const WORKLOADS: &[(&str, &str)] = &[
    ("empty_hook", "bevy::update(|| {});"),
    (
        "velocity_read",
        "bevy::update(|v: Velocity| { let speed = v.x + v.y; });",
    ),
    (
        "velocity_write",
        "bevy::update(|v: Velocity| { v.x += 1.0; });",
    ),
    (
        "transform_read",
        "bevy::update(|t: bevy::Transform| { let ahead = t.translation.x + 1.0; });",
    ),
    (
        "transform_write",
        "bevy::update(|t: bevy::Transform| { t.translation.x += 1.0; });",
    ),
];

fn native(mut movers: Query<&mut Transform, With<Velocity>>) {
    for mut t in &mut movers {
        t.translation.x += 1.0;
    }
}

fn app(source: Option<&str>, entities: usize) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        MimasPlugin::new(Vec::<String>::new()),
    ));
    let script = source.map(|source| {
        app.world_mut()
            .resource_mut::<Assets<MimasScript>>()
            .add(MimasScript {
                path: "bench.mim".into(),
                source: source.into(),
            })
    });
    for _ in 0..entities {
        let mut spawned = app
            .world_mut()
            .spawn((Transform::default(), Velocity { x: 1.0, y: 1.0 }));
        if let Some(script) = &script {
            spawned.insert(Script(script.clone()));
        }
    }
    if script.is_none() {
        app.add_systems(Update, native);
    }
    // compile the script and run every `start` before anything gets timed
    for _ in 0..3 {
        app.update();
    }
    let errors = app.world().resource::<Messages<ScriptError>>();
    assert!(
        errors.is_empty(),
        "the bench script failed to compile or run"
    );
    app
}

fn frames(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame");
    for entities in ENTITIES {
        group.throughput(Throughput::Elements(entities as u64));
        group.bench_with_input(BenchmarkId::new("native", entities), &entities, |b, &n| {
            let mut app = app(None, n);
            b.iter(|| app.update());
        });
        for &(name, source) in WORKLOADS {
            group.bench_with_input(BenchmarkId::new(name, entities), &entities, |b, &n| {
                let mut app = app(Some(source), n);
                b.iter(|| app.update());
            });
        }
    }
    group.finish();
}

criterion_group! {
    name = frame_g;
    config = Criterion::default()
        .sample_size(20)
        .without_plots()
        .measurement_time(Duration::from_secs(3))
        .warm_up_time(Duration::from_secs(1));
    targets = frames
}

criterion_main!(frame_g);
