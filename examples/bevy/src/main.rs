//! A basic brick breaker game that demonstrates how to utilize scripts with the ECS.

use bevy::{camera::ScalingMode, prelude::*, sprite::Anchor};
use mimas::{Literal, Ty, bevy::prelude::*, vm::api::Api};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "mimas breakout".into(),
                        resolution: (WIDTH as u32, HEIGHT as u32).into(),
                        ..window()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: ASSETS.into(),
                    ..default()
                }),
            // loads everything in `assets/scripts`. the web can't list a folder over http, so a
            // web build names the scripts instead
            cfg_select! {
                target_family = "wasm" => MimasPlugin::new([
                    "scripts/ball.mim",
                    "scripts/controls.mim",
                    "scripts/game.mim",
                    "scripts/paddle.mim",
                ]),
                _ => MimasPlugin::default(),
            },
        ))
        .insert_resource(ClearColor(Color::srgb_u8(0x12, 0x14, 0x1d)))
        // components and resources show up in scripts automatically, but messages need registering
        .init_resource::<Game>()
        .script_message::<GameEvent>()
        .script_installer(core)
        .add_systems(Startup, setup)
        .add_systems(Update, (dress_bricks, show_round).after(MimasSystems))
        .run();
}

/// Shares the layout constants with scripts as `core::WIDTH` and friends.
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

/// Given to bevy to initialize the game whenever it's finally ready.
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

    // each script runs on its own entity
    commands.spawn(Script(assets.load("scripts/game.mim")));
    commands.spawn((
        Paddle,
        Script(assets.load("scripts/paddle.mim")),
        Sprite::from_color(
            Color::srgb_u8(0xf6, 0xf8, 0xfb),
            Vec2::new(PADDLE_WIDTH, PADDLE_HEIGHT),
        ),
        Transform::from_xyz(0.0, PADDLE_Y, 1.0),
    ));
    commands.spawn((
        Ball::default(),
        Script(assets.load("scripts/ball.mim")),
        Mesh2d(meshes.add(Circle::new(BALL_RADIUS))),
        MeshMaterial2d(materials.add(Color::srgb_u8(0xff, 0xc2, 0x4b))),
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

/// Gives new bricks a sprite, colored by row.
fn dress_bricks(mut commands: Commands, bricks: Query<(Entity, &Brick), Added<Brick>>) {
    const ROW_COLORS: [Color; 5] = [
        Color::srgb_u8(0x2b, 0x72, 0xd6),
        Color::srgb_u8(0x3d, 0x8e, 0xf7),
        Color::srgb_u8(0x6a, 0xac, 0xf9),
        Color::srgb_u8(0x9c, 0xc3, 0xf7),
        Color::srgb_u8(0x66, 0xe8, 0xff),
    ];
    for (entity, brick) in &bricks {
        let color = ROW_COLORS[brick.row.rem_euclid(ROW_COLORS.len() as i64) as usize];
        commands.entity(entity).insert(Sprite::from_color(
            color,
            Vec2::new(BRICK_WIDTH, BRICK_HEIGHT),
        ));
    }
}

/// Updates the HUD text from `Game`.
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

// shared with the scripts as `core::*`
const WIDTH: f32 = 480.0;
const HEIGHT: f32 = 640.0;
const PADDLE_WIDTH: f32 = 90.0;
const PADDLE_HEIGHT: f32 = 14.0;
const PADDLE_Y: f32 = -280.0;
const BALL_RADIUS: f32 = 7.0;
const BRICK_WIDTH: f32 = 52.0;
const BRICK_HEIGHT: f32 = 20.0;

/// The state of the current round.
#[derive(Reflect, Clone, Copy, PartialEq, Eq, Debug, Default)]
enum State {
    #[default]
    Ready,
    Playing,
    Won,
    Lost,
}

/// The current round, run by `game.mim`.
#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct Game {
    state: State,
    score: i64,
    lives: i64,
    since: f32,
}

/// Marks the paddle entity.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Paddle;

/// The ball's velocity, and if it's waiting to be served.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Ball {
    velocity: Vec2,
    serve: bool,
}

/// A brick. `game.mim` spawns these, and `dress_bricks` gives them a sprite.
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Brick {
    row: i64,
}

/// Events the ball sends to `game.mim`.
#[derive(Message, Reflect, Clone)]
enum GameEvent {
    Broke,
    Dropped,
}

/// Which HUD text an entity is.
#[derive(Component)]
enum Label {
    Score,
    Lives,
    Banner,
}

// the web build runs inside the book, where the page owns the canvas and assets sit beside the
// wasm rather than in an `assets` folder of their own
cfg_select! {
    target_family = "wasm" => {
        const ASSETS: &str = "breakout/assets";

        fn window() -> Window {
            Window {
                canvas: Some("#breakout".into()),
                fit_canvas_to_parent: true,
                // arrow keys and space play the game instead of scrolling the page
                prevent_default_event_handling: true,
                ..default()
            }
        }
    }
    _ => {
        const ASSETS: &str = "assets";

        fn window() -> Window {
            Window::default()
        }
    }
}
