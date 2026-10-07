use std::cell::{Cell, RefCell};

use mimas::{
    Ctx, Literal, Ty, native,
    vm::{
        FnHeader, RtErr, Stashed, Val,
        api::{Api, ModuleApi},
    },
};

use crate::{HEIGHT, WIDTH};

/// What the `luna` natives share with the machine, as a fixture.
#[derive(Default)]
pub(crate) struct Luna {
    pub update: RefCell<Option<Stashed>>,
    pub draw: RefCell<Option<Stashed>>,
    /// Whether a hook is running, which is when registering one faults.
    pub in_hook: Cell<bool>,
    pub tick: Cell<u64>,
}

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    fn hook(
        luna: &mut ModuleApi<'_, '_, '_>,
        name: &'static str,
        doc: &str,
        slot: fn(&Luna) -> &RefCell<Option<Stashed>>,
    ) {
        let unit = Ty::Fn(FnHeader::new(vec![], Ty::Unit, false));
        let parameters = vec![("f".into(), Some(unit))];
        luna.add_described(name, parameters, Ty::Unit, doc, move |ctx, args| {
            let luna = ctx.fixture::<Luna>();
            if luna.in_hook.get() {
                let message = format!("`luna::{name}` registers in top-level code");
                return Err(RtErr::Custom(message));
            }
            *slot(luna).borrow_mut() = Some(ctx.stash(args[0]));
            Ok(Val::Null)
        });
    }

    let mut luna = api.module("luna");
    hook(
        &mut luna,
        "update",
        "Registers `f` as the cart's update, which the console calls 60 times a second before \
         `luna::draw`. It only works in top-level code, and calling it again replaces the update. \
         A cart doesn't need one.

```mimas
struct Game {
    t: int,
}

let game = Game { t = 0 };
luna::update(|| { game.t += 1; });
```",
        |luna| &luna.update,
    );
    hook(
        &mut luna,
        "draw",
        "Registers `f` as the cart's draw, which the console calls right after `luna::update`. \
         Nothing clears the screen between frames, so a draw usually starts with `gfx::clear`. It \
         only works in top-level code, and calling it again replaces the draw. Every cart needs \
         one.

```mimas
luna::draw(|| gfx::clear(Color::Navy));
```",
        |luna| &luna.draw,
    );
    luna.add(tick);
    let doc = "The screen's width in pixels.";
    luna.constant("WIDTH", Ty::Int, Literal::Int(WIDTH as i64), doc);
    let doc = "The screen's height in pixels.";
    luna.constant("HEIGHT", Ty::Int, Literal::Int(HEIGHT as i64), doc);
}

/// Returns how many frames have run since the cart loaded. The count is still `0` during the
/// first frame.
///
/// ```mimas
/// use std::math::ivec2;
///
/// luna::draw(|| {
///     gfx::clear(Color::Black);
///     gfx::text(f"{luna::frame()}", ivec2(2, 2), Color::White);
/// });
/// ```
#[native]
fn tick<'gc>(ctx: Ctx<'gc>) -> i64 {
    ctx.fixture::<Luna>().tick.get() as i64
}
