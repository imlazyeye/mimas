use api::Intrinsic;
use macros::native;
use rand::{prelude::IndexedRandom, seq::SliceRandom};
use shared::Ty;
use vm::{Array, Ctx, DictMap, RtErr, Str, Val, anon, api::Api};

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    api.add_assoc(Ty::Array(Box::new(Ty::Anon(0))), new);
    api.add_assoc(Ty::Array(Box::new(Ty::Anon(0))), new_filled);
    let id = api.add_method(len);
    api.mark_intrinsic(id, Intrinsic::Len);
    let id = api.add_method(contains);
    api.mark_intrinsic(id, Intrinsic::In);
    let id = api.add_method(push);
    api.mark_intrinsic(id, Intrinsic::Push);
    api.add_method(insert);
    api.add_method(pop);
    api.add_method(shuffle);
    api.add_method(extend);
    api.add_method(enumerate);
    api.add_method(flat);
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
    api.add_method(deduped);
    api.add_method(reversed);
    api.add_method(to_dict);
}

/// Creates an empty array. This is the same as writing `[]`, and like `[]` it needs a type
/// annotation if nothing else tells the compiler what it will hold.
///
/// ```mimas
/// let xs: [int] = array::new();
/// xs.push(1); // xs is now [1]
/// ```
#[native]
fn new<'gc>() -> Vec<anon::T<'gc>> {
    Vec::new()
}

/// Creates an array of `len` elements, each a copy of `value`. The copies are deep, so filling
/// with an array gives each slot its own array.
///
/// ```mimas
/// let grid = array::new_filled([0, 0], 2);
/// grid[0].push(1);
/// // grid is [[0, 0, 1], [0, 0]]
/// ```
#[native]
fn new_filled<'gc>(ctx: Ctx<'gc>, value: anon::T<'gc>, len: i64) -> Vec<anon::T<'gc>> {
    (0..len.max(0)) // todo: should be a fault
        .map(|_| anon::Anon(ctx.deep_clone(value.0)))
        .collect()
}

/// Returns the number of elements.
///
/// ```mimas
/// let n = [3, 1, 2].len(); // 3
/// ```
#[native]
fn len(_arr: &[Val<'gc>]) -> usize {
    unreachable!("intrinsics cannot be reached")
}

/// Returns whether any element equals `value`. This is the same check as the
/// [`in` operator](../reference/collections/in-expressions.md).
///
/// ```mimas
/// let xs = [1, 2, 3];
/// let a = xs.contains(2); // true
/// let b = 2 in xs;        // true
/// ```
#[native]
#[allow(unused_variables)]
fn contains(_arr: &[anon::T<'gc>], value: anon::T<'gc>) -> bool {
    unreachable!("intrinsics cannot be reached")
}

/// Appends `value` to the end of the array.
///
/// ```mimas
/// let xs = [1, 2];
/// xs.push(3); // xs is now [1, 2, 3]
/// ```
#[native]
#[allow(unused_variables)]
fn push(_arr: &mut Vec<anon::T<'gc>>, value: anon::T<'gc>) {
    unreachable!("intrinsics cannot be reached")
}

/// Inserts `value` at `index`, moving the element there and everything after it one place toward
/// the end. An `index` equal to the array's length appends.
///
/// An `index` that is negative or past the end is a runtime error.
///
/// ```mimas
/// let xs = [1, 3];
/// xs.insert(1, 2); // xs is now [1, 2, 3]
/// xs.insert(3, 4); // xs is now [1, 2, 3, 4]
/// ```
#[native]
fn insert(arr: &mut Vec<anon::T<'gc>>, index: usize, value: anon::T<'gc>) {
    arr.insert(index, value); // todo: should be a fault
}

/// Removes the last element and returns it, or returns `null` if the array is empty.
///
/// ```mimas
/// let xs = [1, 2];
/// let last = xs.pop(); // 2, and xs is now [1]
/// ```
#[native]
fn pop(arr: &mut Vec<anon::T<'gc>>) -> Option<anon::T<'gc>> {
    arr.pop()
}

/// Puts the elements in a random order.
///
/// ```mimas
/// let deck = [1, 2, 3, 4];
/// deck.shuffle();
/// ```
#[native]
fn shuffle(arr: &mut Vec<anon::T<'gc>>) {
    arr.shuffle(&mut rand::rng());
}

// `other` stays as the gc handle because we already hold a borrow on `arr`. if `other`
// also went through auto-borrow we'd hold two borrows on the same RefLock if the user
// happens to call `arr.extend(arr)`
/// Appends every element of `other`, in order. `other` is unchanged.
///
/// ```mimas
/// let xs = [1, 2];
/// xs.extend([3, 4]); // xs is now [1, 2, 3, 4]
/// ```
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

/// Returns a new array pairing each element with its index.
///
/// ```mimas
/// for (i, name) in ["ant", "bee"].enumerate() {
///     print(f"{i}: {name}"); // 0: ant, then 1: bee
/// }
/// ```
#[native]
fn enumerate(arr: &[anon::T<'gc>]) -> Vec<(usize, anon::T<'gc>)> {
    arr.iter().enumerate().map(|(i, v)| (i, *v)).collect()
}

/// Returns a new array with the elements of each inner array, in order. Only one level is
/// removed: a `[[[int]]]` flattens to a `[[int]]`.
///
/// ```mimas
/// let xs = [[1, 2], [], [3]].flat(); // [1, 2, 3]
/// ```
#[native]
fn flat(arr: Vec<Vec<anon::T<'gc>>>) -> Vec<anon::T<'gc>> {
    arr.into_iter().flatten().collect()
}

/// Returns a random element, or `null` if the array is empty.
///
/// ```mimas
/// let loot = ["sword", "shield", "potion"];
/// let drop = loot.choose() ?? "nothing";
/// ```
#[native]
fn choose(arr: &[Val<'gc>]) -> Option<anon::T<'gc>> {
    arr.choose(&mut rand::rng()).copied().map(anon::Anon)
}

/// Returns the strings joined together with `separator` between each one. Only `[str]` has
/// `join`, so convert other arrays first:
///
/// ```mimas
/// let a = ["a", "b", "c"].join(", "); // "a, b, c"
/// let b = (for n in [1, 2, 3] collect n.to_str()).join("-"); // "1-2-3"
/// ```
#[native]
fn join(arr: &[&str], separator: &str) -> String {
    let parts: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
    parts.join(separator)
}

/// Returns whether the array has no elements.
///
/// ```mimas
/// let xs: [int] = [];
/// let empty = xs.is_empty(); // true
/// ```
#[native]
fn is_empty(arr: &[Val<'gc>]) -> bool {
    arr.is_empty()
}

/// Returns the maximum value present in the array.
///
/// ```mimas
/// let a: [int] = [0, 1, 2];
/// let int_max = a.max(); // 2
///
/// let b: [float] = [0.0, 1.0, 2.0];
/// let float_max = b.max(); // 2.0
/// ```
#[native]
fn max_int(arr: &[i64]) -> Option<i64> {
    arr.iter().max().copied()
}

/// Returns the minimum value present in the array.
///
/// ```mimas
/// let a: [int] = [0, 1, 2];
/// let int_min = a.min(); // 0
///
/// let b: [float] = [0.0, 1.0, 2.0];
/// let float_min = b.min(); // 0.0
/// ```
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

/// Returns the sum of all values in the array.
///
/// ```mimas
/// let a: [int] = [0, 1, 2];
/// let int_sum = a.sum(); // 3
///
/// let b: [float] = [0.0, 1.0, 2.0];
/// let float_sum = b.sum(); // 3.0
/// ```
#[native]
fn sum_int(arr: &[i64]) -> i64 {
    arr.iter().sum()
}

#[native]
fn sum_float(arr: &[f64]) -> f64 {
    arr.iter().sum()
}

/// Sorts the array in ascending order of `keys`, where `keys[i]` is the key for element `i`. Equal
/// keys keep their original order. `keys` is unchanged.
///
/// ```mimas
/// let names = ["cat", "ant", "bee"];
/// names.sort_by_int([3, 1, 2]); // names is now ["ant", "bee", "cat"]
/// ```
///
/// `keys` should be as long as the array. An element without a key is dropped from the array, and
/// extra keys are ignored.
#[native]
fn sort_by_int(arr: &mut Vec<anon::T<'gc>>, keys: Vec<i64>) {
    let mut pairs: Vec<(i64, anon::T<'gc>)> = keys.into_iter().zip(arr.iter().copied()).collect();
    pairs.sort_by_key(|&(k, _)| k);
    *arr = pairs.into_iter().map(|(_, v)| v).collect();
}

/// Sorts the array in ascending order of `keys`, where `keys[i]` is the key for element `i`. Equal
/// keys keep their original order. `keys` is unchanged.
///
/// ```mimas
/// let names = ["far", "near", "mid"];
/// names.sort_by_float([9.5, 0.5, 3.0]); // names is now ["near", "mid", "far"]
/// ```
///
/// `keys` should be as long as the array. An element without a key is dropped from the array, and
/// extra keys are ignored.
#[native]
fn sort_by_float(arr: &mut Vec<anon::T<'gc>>, keys: Vec<f64>) {
    let mut pairs: Vec<(f64, anon::T<'gc>)> = keys.into_iter().zip(arr.iter().copied()).collect();
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    *arr = pairs.into_iter().map(|(_, v)| v).collect();
}

// the indices that would sort `keys` ascending. dispatched on the receiver (int/float key array),
// so each arm is its own typed overload; always returns `[int]` positions.
/// Compares all elements in the array and returns a new array with the indicies sorted. For
/// example, if the maximum value in this array is at index 3, the first element of the returned
/// array will be `3`.
///
/// ```mimas
/// let a = [5, 0, 2, 4];
/// let a_sorted = [0, 3, 2, 1];
///
/// let b = [5.0, 0.0, 2.0, 4.0];
/// let b_sorted = [0, 3, 2, 1];
/// ```
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

/// Replaces the array's contents with the elements at `indices`, in that order. The indices don't
/// have to cover every element once: the array ends up as long as `indices`, and an index can
/// repeat.
///
/// An index that is negative or past the end is a runtime error, and the array is left unchanged.
///
/// ```mimas
/// let xs = ["a", "b", "c"];
/// xs.reorder([2, 0, 1]); // xs is now ["c", "a", "b"]
/// xs.reorder([0, 0]);    // xs is now ["c", "c"]
/// ```
#[native]
fn reorder<'gc>(ctx: Ctx<'gc>, arr: &mut Vec<Val<'gc>>, indices: Vec<i64>) -> Result<(), RtErr> {
    let out = indices
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

/// Returns a new array without repeated elements, keeping the first occurrence of each value in its
/// original order. Elements are compared with `==`, the same check as [`contains`](#contains).
/// Unlike Rust's `dedup`, the repeats don't have to be next to each other. The array itself is
/// unchanged.
///
/// ```mimas
/// let xs = [3, 1, 3, 2, 1];
/// let ys = xs.deduped(); // [3, 1, 2]
/// ```
#[native]
fn deduped(arr: &[anon::T<'gc>]) -> Vec<anon::T<'gc>> {
    let mut kept: Vec<anon::T<'gc>> = Vec::new();
    for value in arr {
        if !kept.iter().any(|k| k.0 == value.0) {
            kept.push(*value);
        }
    }
    kept
}

/// Returns a new array with the elements in reverse order. The array itself is unchanged.
///
/// ```mimas
/// let xs = [1, 2, 3];
/// let ys = xs.reversed(); // [3, 2, 1]
///
/// for x in xs.reversed() {
///     print(x); // 3, then 2, then 1
/// }
/// ```
#[native]
fn reversed(arr: &[anon::T<'gc>]) -> Vec<anon::T<'gc>> {
    arr.iter().rev().copied().collect()
}

/// Returns a dictionary built from `(key, value)` pairs, the reverse of a dictionary's `pairs`.
/// When a key appears more than once, the dictionary holds its last value, in the position where
/// the key first appeared.
///
/// ```mimas
/// let scores = [("ada", 3), ("bob", 5)].to_dict(); // ~{ ada = 3, bob = 5 }
///
/// let names = ["ada", "grace"];
/// let lengths = (for name in names collect (name, name.len())).to_dict();
/// // lengths is ~{ ada = 3, grace = 5 }
/// ```
#[native]
fn to_dict<'gc>(ctx: Ctx<'gc>, arr: Vec<(Str<'gc>, anon::T<'gc>)>) -> anon::DictOf<'gc, 0> {
    let mut dict = DictMap::new();
    for (key, value) in arr {
        dict.insert(key, value.0);
    }
    anon::DictOf(ctx.new_dict(dict))
}
