use gc_arena::{Collect, Gc, Mutation};

use crate::{Ctx, RtResult, Val};

/// A native function callable from mimas. Trait object so we can hold heterogeneous closures
/// (different `Args`/`R` per native) behind a single `Gc<'gc, dyn Native<'gc>>` handle.
///
/// Returns `RtResult<Val>` so natives can surface runtime errors (panic, overflow,
/// io-failure, ...) as `RtError` variants rather than `panic!`-ing through the host.
pub trait Native<'gc>: 'gc {
    fn call(&self, ctx: Ctx<'gc>, args: &[Val<'gc>]) -> RtResult<Val<'gc>>;
}

pub struct WrapFn<F>(pub F);

impl<'gc, F> Native<'gc> for WrapFn<F>
where
    F: Fn(Ctx<'gc>, &[Val<'gc>]) -> RtResult<Val<'gc>> + 'gc,
{
    fn call(&self, ctx: Ctx<'gc>, args: &[Val<'gc>]) -> RtResult<Val<'gc>> {
        (self.0)(ctx, args)
    }
}

// SAFETY: the wrapped closure captures only the user's fn-pointer + phantom-like state; no
// `Gc` pointers live inside the closure itself, so tracing the wrapper is a no-op.
unsafe impl<'gc, F: 'gc> Collect<'gc> for WrapFn<F> {
    const NEEDS_TRACE: bool = false;
}

/// Type alias for the gc'd trait-object handle stored in `State.natives`.
pub type NativeRef<'gc> = Gc<'gc, dyn Native<'gc>>;

/// Allocate a closure inside the arena and coerce to `Gc<dyn Native<'gc>>`.
pub fn make_native<'gc, F>(mc: &Mutation<'gc>, f: F) -> NativeRef<'gc>
where
    F: Fn(Ctx<'gc>, &[Val<'gc>]) -> RtResult<Val<'gc>> + 'gc,
{
    let wrap = WrapFn(f);
    gc_arena::unsize!(Gc::new(mc, wrap) => dyn Native<'gc>)
}
