use api::Intrinsic;
use macros::native;
use rand::{prelude::IndexedRandom, seq::SliceRandom};
use shared::Ty;
use vm::{Array, Ctx, RtErr, Val, anon, api::Api};

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    api.add_assoc(Ty::Array(Box::new(Ty::Anon(0))), new);
    api.add_assoc(Ty::Array(Box::new(Ty::Anon(0))), new_filled);
    let id = api.add_method(len);
    api.mark_intrinsic(id, Intrinsic::Len);
    let id = api.add_method(contains);
    api.mark_intrinsic(id, Intrinsic::In);
    let id = api.add_method(push);
    api.mark_intrinsic(id, Intrinsic::Push);
    api.add_method(pop);
    api.add_method(shuffle);
    api.add_method(extend);
    api.add_method(enumerate);
    api.add_method(flatten);
    api.add_method(choose);
    api.add_method(join);
    api.add_method(is_empty);
    api.add_method_named("max", max_int);
    api.add_method_named("min", min_int);
    api.add_method_named("max", max_float);
    api.add_method_named("min", min_float);
    api.add_method_named("sum", sum_int);
    api.add_method_named("sum", sum_float);
    api.add_method(sort_by_int);
    api.add_method(sort_by_float);
    api.add_method_named("argsort", argsort_int);
    api.add_method_named("argsort", argsort_float);
    api.add_method(reorder);
}

#[native]
fn new<'gc>() -> Vec<anon::T<'gc>> {
    Vec::new()
}

#[native]
fn new_filled<'gc>(ctx: Ctx<'gc>, val: anon::T<'gc>, len: i64) -> Vec<anon::T<'gc>> {
    (0..len.max(0))
        .map(|_| anon::Anon(ctx.deep_clone(val.0)))
        .collect()
}

#[native]
fn len(_arr: &[Val<'gc>]) -> usize {
    unreachable!("intrinsics cannot be reached")
}

#[native]
fn contains(_arr: &[anon::T<'gc>], _val: anon::T<'gc>) -> bool {
    unreachable!("intrinsics cannot be reached")
}

#[native]
fn push(_arr: &mut Vec<anon::T<'gc>>, _val: anon::T<'gc>) {
    unreachable!("intrinsics cannot be reached")
}

#[native]
fn pop(arr: &mut Vec<anon::T<'gc>>) -> Option<anon::T<'gc>> {
    arr.pop()
}

#[native]
fn shuffle(arr: &mut Vec<anon::T<'gc>>) {
    arr.shuffle(&mut rand::rng());
}

// `other` stays as the gc handle because we already hold a borrow on `arr`. if `other`
// also went through auto-borrow we'd hold two borrows on the same RefLock if the user
// happens to call `arr.extend(arr)`
#[native]
fn extend(arr: &mut Vec<Val<'gc>>, other: Array<'gc>) {
    let copy: Vec<Val<'gc>> = match other.0.try_borrow() {
        Ok(v) => v.iter().copied().collect(),
        Err(_) => {
            // presumably the user has tried to extend this array with itself, which is kind of
            // nuts, but technically legal as far as mimas is concerned.
            arr.clone()
        }
    };
    arr.extend(copy);
}

#[native]
fn enumerate(arr: &[anon::T<'gc>]) -> Vec<(usize, anon::T<'gc>)> {
    arr.iter().enumerate().map(|(i, v)| (i, *v)).collect()
}

// todo, should be "flat" or "flattened"
#[native]
fn flatten(arr: Vec<Vec<anon::T<'gc>>>) -> Vec<anon::T<'gc>> {
    arr.into_iter().flatten().collect()
}

#[native]
fn choose(arr: &[Val<'gc>]) -> Option<anon::T<'gc>> {
    arr.choose(&mut rand::rng()).copied().map(anon::Anon)
}

#[native]
fn join(arr: &[&str], sep: &str) -> String {
    let parts: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
    parts.join(sep)
}

#[native]
fn is_empty(arr: &[Val<'gc>]) -> bool {
    arr.is_empty()
}

#[native]
fn max_int(arr: &[i64]) -> Option<i64> {
    arr.iter().max().copied()
}

#[native]
fn min_int(arr: &[i64]) -> Option<i64> {
    arr.iter().min().copied()
}

#[native]
fn max_float(arr: &[f64]) -> Option<f64> {
    arr.iter().copied().max_by(f64::total_cmp)
}

#[native]
fn min_float(arr: &[f64]) -> Option<f64> {
    arr.iter().copied().min_by(f64::total_cmp)
}

#[native]
fn sum_int(arr: &[i64]) -> i64 {
    arr.iter().sum()
}

#[native]
fn sum_float(arr: &[f64]) -> f64 {
    arr.iter().sum()
}

#[native]
fn sort_by_int(arr: &mut Vec<anon::T<'gc>>, keys: Vec<i64>) {
    let mut pairs: Vec<(i64, anon::T<'gc>)> = keys.into_iter().zip(arr.iter().copied()).collect();
    pairs.sort_by_key(|&(k, _)| k);
    *arr = pairs.into_iter().map(|(_, v)| v).collect();
}

#[native]
fn sort_by_float(arr: &mut Vec<anon::T<'gc>>, keys: Vec<f64>) {
    let mut pairs: Vec<(f64, anon::T<'gc>)> = keys.into_iter().zip(arr.iter().copied()).collect();
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    *arr = pairs.into_iter().map(|(_, v)| v).collect();
}

// the indices that would sort `keys` ascending. dispatched on the receiver (int/float key array),
// so each arm is its own typed overload; always returns `[int]` positions.
#[native]
fn argsort_int(keys: &[i64]) -> Vec<i64> {
    let mut idx: Vec<i64> = (0..keys.len() as i64).collect();
    idx.sort_by_key(|&i| keys[i as usize]);
    idx
}

#[native]
fn argsort_float(keys: &[f64]) -> Vec<i64> {
    let mut idx: Vec<i64> = (0..keys.len() as i64).collect();
    idx.sort_by(|&a, &b| keys[a as usize].total_cmp(&keys[b as usize]));
    idx
}

#[native]
fn reorder<'gc>(ctx: Ctx<'gc>, arr: &mut Vec<Val<'gc>>, perm: Vec<i64>) -> Result<(), RtErr> {
    let out = perm
        .iter()
        .map(|&i| {
            usize::try_from(i)
                .ok()
                .and_then(|u| arr.get(u).copied())
                .ok_or(RtErr::IndexOutOfBounds)
        })
        .collect::<Result<Vec<_>, _>>()?;
    *arr = out;
    Ok(())
}
