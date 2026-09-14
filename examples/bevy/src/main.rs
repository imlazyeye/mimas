//! A basic brick breaker game that demonstrates how to utilize scripts with the ECS.

use bevy::{camera::ScalingMode, prelude::*, sprite::Anchor};
use mimas::{Literal, Ty, bevy::prelude::*, vm::api::Api};

// the geometry Rust draws with, which the scripts read as `core::*`
const WIDTH: f32 = 480.0;
const HEIGHT: f32 = 640.0;
const PADDLE_WIDTH: f32 = 90.0;
const PADDLE_HEIGHT: f32 = 14.0;
const PADDLE_Y: f32 = -280.0;
const BALL_RADIUS: f32 = 7.0;
const BRICK_WIDTH: f32 = 52.0;
const BRICK_HEIGHT: f32 = 20.0;

/// Where the round is. Scripts see it because `Game` holds one.
#[derive(Reflect, Clone, Copy, PartialEq, Eq, Debug, Default)]
enum State {
    #[default]
    Ready,
    Playing,
    Won,
    Lost,
}

/// The round, owned by `game.mim`.
#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct Game {
    state: State,
    score: i64,
    lives: i64,
    since: f32,
}

/// Marks the paddle, so `ball.mim` finds it with `Paddle::entities()`.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Paddle;

/// The ball's motion. `serve` is set while it waits on the paddle for the round to start.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Ball {
    velocity: Vec2,
    serve: bool,
}

/// A brick, spawned by `game.mim` and given a sprite here.
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Brick {
    row: i64,
}

/// What the ball reports to `game.mim`.
#[derive(Message, Reflect, Clone)]
enum GameEvent {
    Broke,
    Dropped,
}

/// The HUD texts, redrawn from `Game`.
#[derive(Component)]
enum Label {
    Score,
    Lives,
    Banner,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "mimas breakout".into(),
                    resolution: (WIDTH as u32, HEIGHT as u32).into(),
                    ..default()
                }),
                ..default()
            }),
            // loads every `.mim` under `assets/scripts`, and `controls.mim` declares a module, so
            // it compiles into the other three
            MimasPlugin::default(),
        ))
        .insert_resource(ClearColor(Color::srgb(0.09, 0.09, 0.13)))
        // components and resources that derive `Reflect` reach scripts on their own, messages
        // need a line
        .init_resource::<Game>()
        .script_message::<GameEvent>()
        .script_installer(core)
        .add_systems(Startup, setup)
        .add_systems(Update, (dress_bricks, show_round).after(MimasSystems))
        .run();
}

/// Publishes the geometry as `core::WIDTH` and friends.
fn core(api: &mut Api) {
    let mut core = api.module("core");
    for (name, value) in [
        ("WIDTH", WIDTH),
        ("HEIGHT", HEIGHT),
        ("PADDLE_WIDTH", PADDLE_WIDTH),
        ("PADDLE_HEIGHT", PADDLE_HEIGHT),
        ("PADDLE_Y", PADDLE_Y),
        ("BALL_RADIUS", BALL_RADIUS),
        ("BRICK_WIDTH", BRICK_WIDTH),
        ("BRICK_HEIGHT", BRICK_HEIGHT),
    ] {
        core.constant(name, Ty::Float, Literal::Float(value as f64), "");
    }
}

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: WIDTH,
                min_height: HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));

    // each script rides on an entity, and its hooks take that entity's components as parameters
    commands.spawn(Script(assets.load("scripts/game.mim")));
    commands.spawn((
        Paddle,
        Script(assets.load("scripts/paddle.mim")),
        Sprite::from_color(Color::WHITE, Vec2::new(PADDLE_WIDTH, PADDLE_HEIGHT)),
        Transform::from_xyz(0.0, PADDLE_Y, 1.0),
    ));
    commands.spawn((
        Ball::default(),
        Script(assets.load("scripts/ball.mim")),
        Mesh2d(meshes.add(Circle::new(BALL_RADIUS))),
        MeshMaterial2d(materials.add(Color::WHITE)),
        Transform::from_xyz(0.0, PADDLE_Y + PADDLE_HEIGHT / 2.0 + BALL_RADIUS, 1.0),
    ));

    let corner = Vec2::new(WIDTH / 2.0 - 12.0, HEIGHT / 2.0 - 8.0);
    for (label, anchor, at) in [
        (
            Label::Score,
            Anchor::TOP_LEFT,
            Vec2::new(-corner.x, corner.y),
        ),
        (Label::Lives, Anchor::TOP_RIGHT, corner),
        (Label::Banner, Anchor::CENTER, Vec2::new(0.0, -80.0)),
    ] {
        commands.spawn((
            label,
            Text2d::default(),
            TextFont::from_font_size(28.0),
            TextLayout::justify(Justify::Center),
            anchor,
            Transform::from_translation(at.extend(5.0)),
        ));
    }
}

/// Gives each brick `game.mim` spawned a sprite, colored by row.
fn dress_bricks(mut commands: Commands, bricks: Query<(Entity, &Brick), Added<Brick>>) {
    const ROW_COLORS: [Color; 5] = [
        Color::srgb(0.93, 0.33, 0.31),
        Color::srgb(0.95, 0.61, 0.24),
        Color::srgb(0.96, 0.85, 0.32),
        Color::srgb(0.45, 0.80, 0.42),
        Color::srgb(0.36, 0.62, 0.93),
    ];
    for (entity, brick) in &bricks {
        let color = ROW_COLORS[brick.row.rem_euclid(ROW_COLORS.len() as i64) as usize];
        commands.entity(entity).insert(Sprite::from_color(
            color,
            Vec2::new(BRICK_WIDTH, BRICK_HEIGHT),
        ));
    }
}

/// Draws the score, the lives, and the banner from the round.
fn show_round(game: Res<Game>, mut texts: Query<(&Label, &mut Text2d)>) {
    for (label, mut text) in &mut texts {
        let wanted = match label {
            Label::Score => game.score.to_string(),
            Label::Lives => "o "
                .repeat(game.lives.max(0) as usize)
                .trim_end()
                .to_string(),
            Label::Banner => match game.state {
                State::Ready => "press space".to_string(),
                State::Playing => String::new(),
                State::Won => format!("cleared with {} points\npress space", game.score),
                State::Lost => "game over\npress space".to_string(),
            },
        };
        if text.0 != wanted {
            text.0 = wanted;
        }
    }
}
