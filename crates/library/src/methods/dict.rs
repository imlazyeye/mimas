use macros::native;
use vm::{Ctx, DictMap, Str, anon, api::Api};

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    api.add_method(get);
    api.add_method(len);
    api.add_method(contains_key);
    api.add_method(insert);
    api.add_method(remove);
    api.add_method(pairs);
}

/// Returns the value stored under `key`, or `null` if there isn't one. This is the same as
/// indexing with `dict[key]`.
///
/// ```mimas
/// let ages = ~{ ada = 36 };
/// let a = ages.get("ada"); // 36
/// let b = ages.get("bob"); // null
/// ```
#[native]
fn get(d: &DictMap<'gc, anon::T<'gc>>, key: Str<'gc>) -> Option<anon::T<'gc>> {
    d.get(&key).copied()
}

/// Returns the number of entries.
///
/// ```mimas
/// let n = ~{ a = 1, b = 2 }.len(); // 2
/// ```
#[native]
fn len(d: &DictMap<'gc>) -> usize {
    d.len()
}

/// Returns whether the dictionary has an entry for `key`. Unlike indexing, this tells a missing
/// key apart from one whose value is `null`.
///
/// ```mimas
/// let slots: ~{int?} = ~{ a = null };
/// let a = slots.contains_key("a"); // true, though slots["a"] is null
/// let b = slots.contains_key("b"); // false
/// ```
#[native]
fn contains_key<'gc>(ctx: Ctx<'gc>, d: &DictMap<'gc>, key: String) -> bool {
    // intern to normalize: `Str` uses pointer-identity Eq/Hash, so we need the same Gc
    // handle that's already in the dict's keys.
    let key = ctx.intern(&key);
    d.contains_key(&key)
}

/// Sets `key` to `value` and returns the value it replaced, or `null` if the key is new. A new key
/// is added at the end of the iteration order. Replacing a value doesn't move its key.
///
/// Assigning with `dict[key] = value` does the same without returning anything.
///
/// ```mimas
/// let stock = ~{ apples = 3 };
/// let old = stock.insert("apples", 5); // 3
/// let new = stock.insert("pears", 2);  // null
/// ```
#[native]
fn insert(
    d: &mut DictMap<'gc, anon::T<'gc>>,
    key: Str<'gc>,
    value: anon::T<'gc>,
) -> Option<anon::T<'gc>> {
    d.insert(key, value)
}

/// Removes the entry for `key` and returns its value, or returns `null` if there was no such
/// entry. The remaining entries keep their order.
///
/// ```mimas
/// let stock = ~{ apples = 3, pears = 2 };
/// let a = stock.remove("apples"); // 3, and stock is now ~{ pears = 2 }
/// let b = stock.remove("plums");  // null
/// ```
#[native]
fn remove(d: &mut DictMap<'gc, anon::T<'gc>>, key: Str<'gc>) -> Option<anon::T<'gc>> {
    d.remove(&key)
}

/// Returns every entry as a `(key, value)` tuple, in the order the keys were first inserted.
///
/// ```mimas
/// let scores = ~{ ada = 3, bob = 5 };
/// let entries = scores.pairs(); // [("ada", 3), ("bob", 5)]
/// ```
///
/// A `for` loop over the dictionary visits the same pairs without building an array:
///
/// ```mimas
/// for (name, score) in scores {
///     print(f"{name} has {score}");
/// }
/// ```
#[native]
fn pairs(d: &DictMap<'gc, anon::T<'gc>>) -> Vec<(Str<'gc>, anon::T<'gc>)> {
    d.iter().map(|(&k, &v)| (k, v)).collect()
}
