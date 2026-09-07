use macros::native;
use vm::{Ctx, DictMap, Str, anon, api::Api};

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    api.add_method(len);
    api.add_method(contains_key);
    api.add_method(insert);
    api.add_method(remove);
    api.add_method(pairs);
}

#[native]
fn len(d: &DictMap<'gc>) -> usize {
    d.len()
}

#[native]
fn contains_key<'gc>(ctx: Ctx<'gc>, d: &DictMap<'gc>, key: String) -> bool {
    // intern to normalize: `Str` uses pointer-identity Eq/Hash, so we need the same Gc
    // handle that's already in the dict's keys.
    let key = ctx.intern(&key);
    d.contains_key(&key)
}

#[native]
fn insert(
    d: &mut DictMap<'gc, anon::T<'gc>>,
    key: Str<'gc>,
    val: anon::T<'gc>,
) -> Option<anon::T<'gc>> {
    d.insert(key, val)
}

#[native]
fn remove(d: &mut DictMap<'gc, anon::T<'gc>>, key: Str<'gc>) -> Option<anon::T<'gc>> {
    d.remove(&key)
}

#[native]
fn pairs(d: &DictMap<'gc, anon::T<'gc>>) -> Vec<(Str<'gc>, anon::T<'gc>)> {
    d.iter().map(|(&k, &v)| (k, v)).collect()
}
