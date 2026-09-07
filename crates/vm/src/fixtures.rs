//! Fixtures are our take on the per-interpreter "singleton" storage in [Catherine West's
//! fabricator](https://github.com/kyren/fabricator) (`Registry::singleton` in its vm crate) --
//! which is itself the old type-keyed-storage idiom you may know as `anymap`, `http::Extensions`,
//! or Bevy's `Resources`. We use our own name partly because we've reshaped it (departures at the
//! bottom), and partly because "singleton" oversells it: there's one per *type per Vm*, not one
//! per program.
//!
//! Similar to the story in freeze.rs, Fixtures are inspired by a series of other libraries:
//! Catherine West's `Singleton` in [fabricator](https://github.com/kyren/fabricator), Bevy's
//! [Resource](https://docs.rs/bevy_ecs/latest/bevy_ecs/system/trait.Resource.html), `http::Extensions`, and the broader type-keyed storage idiom of `anymap`.
//!
//! [freeze](crate::freeze) explained itself with a coat check at a party, so let's stay in the
//! venue. We made a big assumption back there: "how do we know coat check exists?"
//!
//! Oh yeah, if you thought the last metaphor fell apart, buckle up for this one.
//!
//! A native fn runs deep inside the party with nothing in its pockets but a [Ctx](crate::Ctx).
//! The host, meanwhile, is standing outside the party entirely. If the two want to share anything
//! that isn't a gc value -- a [FreezeCell](crate::freeze::FreezeCell), an RNG, the script's args
//! -- it has to live somewhere *both* can find. That somewhere is the venue's lobby, and the
//! things in it are its fixtures. The one we care about? Why, the coat check of course!
//!
//! But just like with Freeze, there's some promises and assumptions we have to make for this to
//! work.
//!
//! 1. There is exactly _one_ coat check. Imagine if there was more than one, it'd be chaos! If we
//!    can agree there's only one we can all know what we mean when we say "the coat check".
//! 2. Fixtures assemble themselves the first time anyone asks. The coat check attendants, as hard
//!    working as they are, do not live in the coat check; they show up and open it when guests
//!    actually arrive to hand over coats. That's the `Default` bound plus `entry().or_insert_with`
//!    in [Fixtures::get]: there's no install step to forget and no ordering bug where a native asks
//!    before the attendants are ready. Either side can be first.
//! 3. Fixtures are bolted to the building. Once one exists it never moves and is never torn down
//!    until the whole venue is -- entries are never removed, and each one is `Box`ed so it keeps
//!    its own patch of floor even when the lobby's registry (the HashMap) reshuffles itself. That's
//!    why it's safe!
//!
//! There, isn't that better? That was definitely worth it, again.

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
};

/// Marker for types that can live as a per-Vm fixture. Must be `Default`-constructible and
/// `'static` -- see the module docs for why both bounds are load-bearing.
pub trait Fixture: Default + Any + 'static {}
impl<T: Default + Any + 'static> Fixture for T {}

/// Per-Vm type-keyed storage for host-shared state. Lazily creates entries on first access via
/// `T::default()`. Designed to hold things like [FreezeCell](crate::freeze::FreezeCell)s that
/// host code installs `&mut T` borrows into during mimas execution.
#[derive(Default)]
pub struct Fixtures {
    cells: RefCell<HashMap<TypeId, Box<dyn Any>>>,
}

// SAFETY: Fixtures holds only `'static` heap-boxed values, none containing Gc handles.
unsafe impl<'gc> gc_arena::Collect<'gc> for Fixtures {
    const NEEDS_TRACE: bool = false;
}

impl Fixtures {
    pub fn get<T: Fixture>(&self) -> &T {
        let tid = TypeId::of::<T>();
        let ptr: *const T = {
            let mut cells = self.cells.borrow_mut();
            let entry = cells.entry(tid).or_insert_with(|| Box::new(T::default()));
            entry
                .downcast_ref::<T>()
                .expect("fixture type mismatch (TypeId collision?)") as *const T
        };
        // SAFETY: entries are never removed; Box's heap address is stable for the
        // lifetime of self. The returned reference is tied to &self via inference.
        unsafe { &*ptr }
    }
}
