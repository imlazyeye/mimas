//! A very basic game loop used to demonstrate how to use Fixtures and FreezeCells.

use mimas::{
    mimas,
    vm::{
        Ctx,
        freeze::{Freeze, FreezeCell},
    },
};

/// Our teeny little controller for our game.
struct Game {
    score: i64,
}

/// The box we stuff our Game within. This is how we can safely share it with mimas!
type GameCell = FreezeCell<Freeze![&'freeze mut Game]>;

/// An extension trait for the normal Ctx so that we can make our common patterns a bit more
/// comfortable.
trait GameCtx<'gc> {
    fn game<R>(self, f: impl FnOnce(&mut Game) -> R) -> R;
}

impl<'gc> GameCtx<'gc> for Ctx<'gc> {
    /// Takes a closure that can safely access Game during its life.
    fn game<R>(self, f: impl FnOnce(&mut Game) -> R) -> R {
        self.fixture::<GameCell>().with_mut(|g| f(g)).unwrap()
    }
}

/// Here mimas can reach into the true Game and update the score.
#[mimas(game)]
fn add_score(ctx: Ctx, n: i64) -> i64 {
    ctx.game(|g| {
        g.score += n;
        g.score
    })
}

const SOURCE: &str = r#"
use game;

fn update() {
    game::add_score(10);
}
"#;

fn main() {
    let mut game = Game { score: 0 };
    let mut vm = mimas::compile_source(SOURCE).unwrap();
    vm.run().unwrap();

    let game_cell = vm.fixture::<GameCell>();

    for _ in 0..3 {
        // freeze our game -- we guarantee it's safe and sound while mimas does its thing.
        game_cell.freeze(&mut game, || vm.call_fn("update").unwrap());
    }

    assert_eq!(game.score, 30);
    println!("We beat the game!");
}
