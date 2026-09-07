//! The Freeze pattern comes from Catherine West: it originates in [piccolo's `freeze`
//! module](https://docs.rs/piccolo-util/latest/piccolo_util/freeze/index.html) as "a general way
//! of safely erasing a single lifetime parameter from a type", and our version follows the
//! slimmer form from another project by Catherine, [fabricator](https://github.com/kyren/fabricator).
//!
//! It is remarkably clever but took me a while to fully wrap my head around, so I've now written
//! myself a silly metaphor explaining it to come back to when I get confused.
//!
//! The garbage collector in our Vm has a rule: _everything_ it manages must live forever ('static).
//! However, sometimes a script needs to reach state that lives *outside* the Vm and that the Vm
//! doesn't own -- a mutable handle into the host program, say. That borrow is short-lived, the Vm
//! is long-lived. We can't stash a short-lived borrow inside a long-lived Vm the normal way,
//! because the Vm outlives it.
//!
//! So instead of storing it, we slip past the rules and just lend it. We'll give it back. Promise.
//!
//! Imagine Freeze as a coat check at a party.
//!
//! 1. We walk up to the coat check counter, hand over our coat, and are given back a ticket that
//!    tells us "your coat is in slot #5 and will stay there while you're here." That's what
//!    [FreezeCell::freeze] is: we hand it our coat (`v`) and we enter the party (`body`).
//! 2. We enjoy ourselves at the party. We take solace in the knowledge that coat check is keeping
//!    our coat safe and sound.
//! 3. It's time to leave the party, and even though we can at times be forgetful, the coat check
//!    attendant very diligently shoves our coat right in front of us -- there's no way we could
//!    possibly leave without it. That's the `Guard`'s Drop.
//!
//! That's the whole point -- our coat is only inside of coat check while we're at the party, and
//! there's no way for us to leave without it. Similarly, the "lent" value from Vm is only with us
//! while we're inside of the `body`. The "lives forever" promise the Vm requires is never
//! observable -- by the time the garbage collector runs again, it's right where it left the value.
//! Nothing's in coat check.
//!
//! There are two rules the coat check attendants enforce to make sure everyone's coats remain safe
//! (or in other words, these are the safety measures Freeze takes so that its unsafe code is always
//! sound).
//!
//! 1. No Gc pointers inside of coat check. The cell is marked `NEEDS_TRACE = false`, so the
//!    collector skips right over it. That's fine for a host borrow: the GC doesn't own it, so
//!    there's nothing to trace, and the host is the one keeping it alive for us. But a Gc handle
//!    hidden in here would be invisible to the collector, which would happily free it out from
//!    under us -- a dangling pointer the moment we touch it again. So only host references go in,
//!    never anything the GC owns.
//!
//! 2. Other guests can glance at your coat, but they can't walk off with it. While you're at the
//!    party, code inside `body` can ask coat check for a look -- that's `with` / `with_mut`. Those
//!    callbacks are typed `for<'f> FnOnce(&Frozen<'f>)`, so they must work for *any* lifetime: they
//!    can eye the coat on the spot but can't pocket a reference and carry it out past the `body`
//!    scope. The only one who ever leaves with the coat is you, on your way out (step 3).
//!
//! There, good, that definitively wasn't more confusing than the code itself!

use std::{cell::RefCell, marker::PhantomData, mem};

pub trait Freeze<'f> {
    type Frozen: 'f;
}

pub struct DynFreeze<T: ?Sized>(PhantomData<T>);

impl<'f, T: ?Sized + for<'a> Freeze<'a>> Freeze<'f> for DynFreeze<T> {
    type Frozen = <T as Freeze<'f>>::Frozen;
}

#[macro_export]
#[doc(hidden)]
macro_rules! __freeze_Freeze {
    ($f:lifetime => $frozen:ty) => {
        $crate::freeze::DynFreeze::<
            dyn for<$f> $crate::freeze::Freeze<$f, Frozen = $frozen>,
        >
    };
    ($frozen:ty) => {
        $crate::__freeze_Freeze!['freeze => $frozen]
    };
}

pub use crate::__freeze_Freeze as Freeze;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FreezeError {
    Expired,
    Borrowed,
}

pub struct FreezeCell<F: for<'f> Freeze<'f>> {
    cell: RefCell<Option<<F as Freeze<'static>>::Frozen>>,
}

impl<F: for<'f> Freeze<'f>> Default for FreezeCell<F> {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: FreezeCell only stores lifetime-erased host references, never a Gc handle.
unsafe impl<'gc, F: for<'f> Freeze<'f>> gc_arena::Collect<'gc> for FreezeCell<F> {
    const NEEDS_TRACE: bool = false;
}

impl<F: for<'f> Freeze<'f>> FreezeCell<F> {
    pub const fn new() -> Self {
        Self {
            cell: RefCell::new(None),
        }
    }

    pub fn freeze<'f, R>(&self, v: <F as Freeze<'f>>::Frozen, body: impl FnOnce() -> R) -> R {
        // SAFETY: three invariants, mirroring fabricator-util's freeze.rs:
        // 1: the 'static lifetime is a lie outside code can't observe -- `with[_mut]` callbacks
        //    must work for any lifetime, so they can't depend on it.
        // 2: `Guard::drop` restores the previous value before our scope ends.
        // 3: if the guard can't restore (cell still borrowed), we abort rather than leave the stale
        //    lifetime-erased value visible to later borrows.
        let next = unsafe {
            mem::transmute::<<F as Freeze<'f>>::Frozen, <F as Freeze<'static>>::Frozen>(v)
        };

        let prev = self
            .cell
            .try_borrow_mut()
            .expect("FreezeCell::freeze cannot be called inside with[_mut]")
            .replace(next);

        struct Guard<'a, F: for<'f> Freeze<'f>> {
            cell: &'a RefCell<Option<<F as Freeze<'static>>::Frozen>>,
            prev: Option<<F as Freeze<'static>>::Frozen>,
        }

        impl<F: for<'f> Freeze<'f>> Drop for Guard<'_, F> {
            fn drop(&mut self) {
                if let Ok(mut cell) = self.cell.try_borrow_mut() {
                    *cell = self.prev.take();
                } else {
                    eprintln!("freeze lock held during guard drop, aborting!");
                    std::process::abort();
                }
            }
        }

        let _g = Guard::<F> {
            cell: &self.cell,
            prev,
        };

        body()
    }

    pub fn with<R>(
        &self,
        f: impl for<'f> FnOnce(&<F as Freeze<'f>>::Frozen) -> R,
    ) -> Result<R, FreezeError> {
        let v = self.cell.try_borrow().map_err(|_| FreezeError::Borrowed)?;
        Ok(f(v.as_ref().ok_or(FreezeError::Expired)?))
    }

    pub fn with_mut<R>(
        &self,
        f: impl for<'f> FnOnce(&mut <F as Freeze<'f>>::Frozen) -> R,
    ) -> Result<R, FreezeError> {
        let mut v = self
            .cell
            .try_borrow_mut()
            .map_err(|_| FreezeError::Borrowed)?;
        Ok(f(v.as_mut().ok_or(FreezeError::Expired)?))
    }
}
