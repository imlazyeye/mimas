use std::cell::{RefCell, RefMut};

use rand::{SeedableRng, rngs::StdRng};

/// Holds the random seed for a Vm. A host that wants a script to get the same ones each time seeds
/// it with `vm.fixture::<Random>().seed(n)`.
pub struct Random(RefCell<StdRng>);

impl Random {
    /// Restarts the generator from `seed`.
    ///
    /// ## Note
    /// A seed gives the same numbers on every target, but not across versions of mimas that update
    /// the generator.
    pub fn seed(&self, seed: u64) {
        *self.0.borrow_mut() = StdRng::seed_from_u64(seed);
    }

    pub(crate) fn rng(&self) -> RefMut<'_, StdRng> {
        self.0.borrow_mut()
    }
}

impl Default for Random {
    fn default() -> Self {
        Self(RefCell::new(rand::make_rng()))
    }
}
