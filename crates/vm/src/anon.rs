use crate::{Array, Dict, DictMap, Val};

/// Placeholder parameter slots for native sigs. Each unique `Anon<N>` in a sig is freshened to a
/// distinct `Ty::Vid` per call site, with the same `N` mapped to the same vid within that call --
/// the foundation generics would lower onto.
///
/// Lives in `vm` rather than `api` because it wraps [`Val<'gc>`], which carries the arena
/// lifetime -- `api` can't depend on `vm`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct Anon<'gc, const N: u32>(pub Val<'gc>);

/// Registration view of a gc array whose elements are placeholder slot `N`. The macro rewrites
/// anon-element array borrows (`&[anon::T<'gc>]`, `&mut Vec<anon::U<'gc>>`, ...) to this so the
/// registered type is `Array(Anon(N))` -- the bare `Array<'gc>` handle would erase `N` to 0.
pub struct ArrayOf<'gc, const N: u32>(pub Array<'gc>);

/// Registration view of a gc dict whose values are placeholder slot `N` -- the dict analogue of
/// [`ArrayOf`].
pub struct DictOf<'gc, const N: u32>(pub Dict<'gc>);

/// Reinterpret a gc array's backing `Vec<Val>` as `&mut Vec<Anon<N>>`, so a `#[native]` can take a
/// type-safe-but-generic `&mut Vec<anon::T<'gc>>` and mutate the array in place. Unlike a real
/// typed element (`&mut Vec<i64>`), this needs no copy-in/out -- `Anon` *is* a `Val`.
pub fn as_vec_mut<'a, 'gc, const N: u32>(vals: &'a mut Vec<Val<'gc>>) -> &'a mut Vec<Anon<'gc, N>> {
    // SAFETY: `Anon<N>` is `#[repr(transparent)]` over `Val<'gc>`, so `Vec<Anon<N>>` and `Vec<Val>`
    // are layout-identical; this reinterprets only the element type and preserves the borrow.
    unsafe { &mut *(vals as *mut Vec<Val<'gc>> as *mut Vec<Anon<'gc, N>>) }
}

/// Read-only twin of [`as_vec_mut`]. Returns `&Vec` (not `&[..]`) so the macro's rebinding can
/// coerce to either of the user's written forms (`&[anon::T<'gc>]` or `&Vec<anon::T<'gc>>`).
#[allow(clippy::ptr_arg)]
pub fn as_vec_ref<'a, 'gc, const N: u32>(vals: &'a Vec<Val<'gc>>) -> &'a Vec<Anon<'gc, N>> {
    // SAFETY: as in `as_vec_mut`; preserves the borrow.
    unsafe { &*(vals as *const Vec<Val<'gc>> as *const Vec<Anon<'gc, N>>) }
}

/// Reinterpret a gc dict's `DictMap<'gc, Val>` as `&DictMap<'gc, Anon<N>>` -- the dict analogue of
/// [`as_vec_mut`], for read-only typed views.
pub fn as_dict_ref<'a, 'gc, const N: u32>(d: &'a DictMap<'gc>) -> &'a DictMap<'gc, Anon<'gc, N>> {
    // SAFETY: `Anon<N>` is `#[repr(transparent)]` over `Val<'gc>`, so `DictMap<'gc, Anon<N>>` and
    // `DictMap<'gc, Val>` are layout-identical; this reinterprets only the value slot.
    unsafe { &*(d as *const DictMap<'gc> as *const DictMap<'gc, Anon<'gc, N>>) }
}

/// Reinterpret a gc dict's `DictMap<'gc, Val>` as `&mut DictMap<'gc, Anon<N>>`, so a `#[native]`
/// can take a type-safe-but-generic `&mut DictMap<'gc, anon::T<'gc>>` and mutate the dict in
/// place -- the dict analogue of [`as_vec_mut`].
pub fn as_dict_mut<'a, 'gc, const N: u32>(
    d: &'a mut DictMap<'gc>,
) -> &'a mut DictMap<'gc, Anon<'gc, N>> {
    // SAFETY: as in `as_dict_ref`; preserves the borrow.
    unsafe { &mut *(d as *mut DictMap<'gc> as *mut DictMap<'gc, Anon<'gc, N>>) }
}

/// Placeholder parameter type for native signatures. Can represent any type but must be the same as
/// every other `T` within the context of this specific signature.
pub type T<'gc> = Anon<'gc, 0>;

/// Placeholder parameter type for native signatures. Can represent any type but must be the same as
/// every other `U` within the context of this specific signature.
pub type U<'gc> = Anon<'gc, 1>;

/// Placeholder parameter type for native signatures. Can represent any type but must be the same as
/// every other `V` within the context of this specific signature.
pub type V<'gc> = Anon<'gc, 2>;

/// Placeholder parameter type for native signatures. Can represent any type but must be the same as
/// every other `W` within the context of this specific signature.
pub type W<'gc> = Anon<'gc, 3>;
