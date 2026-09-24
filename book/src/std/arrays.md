# Arrays

The methods and associated functions available on every array. For the array type itself, literals, and indexing, see [Arrays](../reference/collections/arrays.md) in the language reference.

```admonish info title="Reading the signatures"
`T` stands for the array's element type. It isn't a type you can write in mimas: it means whatever the array holds, so `push` on an `[int]` takes an `int`.

Most methods work on any array. A signature that writes out `self`'s type, like `self: [int]`, only exists on arrays of that type.
```

| Item | Summary |
| :-- | :-- |
| [`array::new`](#arraynew) | Creates an empty array |
| [`array::new_filled`](#arraynew_filled) | Creates an array holding copies of one value |
| [`argsort`](#argsort) | The indices that would sort the array |
| [`choose`](#choose) | A random element |
| [`contains`](#contains) | Whether the array holds a value |
| [`enumerate`](#enumerate) | Each element paired with its index |
| [`extend`](#extend) | Appends another array |
| [`flatten`](#flatten) | Joins an array of arrays into one |
| [`is_empty`](#is_empty) | Whether the array has no elements |
| [`join`](#join) | Joins strings with a separator |
| [`len`](#len) | The number of elements |
| [`max`](#max) | The largest number |
| [`min`](#min) | The smallest number |
| [`pop`](#pop) | Removes and returns the last element |
| [`push`](#push) | Appends an element |
| [`reorder`](#reorder) | Rearranges the elements by index |
| [`shuffle`](#shuffle) | Randomizes the order |
| [`sort_by_float`](#sort_by_float) | Sorts by a matching array of `float` keys |
| [`sort_by_int`](#sort_by_int) | Sorts by a matching array of `int` keys |
| [`sum`](#sum) | Adds the numbers together |

## Associated Functions

### `array::new`

```mimas
fn new() -> [T]
```

Creates an empty array. This is the same as writing `[]`, and like `[]` it needs a type annotation if nothing else tells the compiler what it will hold.

```mimas
let xs: [int] = array::new();
xs.push(1); // xs is now [1]
```

### `array::new_filled`

```mimas
fn new_filled(value: T, len: int) -> [T]
```

Creates an array of `len` elements, each a copy of `value`. The copies are deep, so filling with an array gives each slot its own array. A negative `len` gives an empty array.

```mimas
let grid = array::new_filled([0, 0], 2);
grid[0].push(1);
// grid is [[0, 0, 1], [0, 0]]
```

## Methods

### `argsort`

```mimas
fn argsort(self: [int]) -> [int]
fn argsort(self: [float]) -> [int]
```

Returns the indices that would sort the array in ascending order. Equal values keep their original order. The array itself is unchanged.

Pair it with [`reorder`](#reorder) to put another array in the same order:

```mimas
let names = ["cat", "ant", "bee"];
let ages = [7, 2, 4];
let order = ages.argsort(); // [1, 2, 0]
names.reorder(order);       // names is ["ant", "bee", "cat"]
```

### `choose`

```mimas
fn choose(self) -> T?
```

Returns a random element, or `null` if the array is empty.

```mimas
let loot = ["sword", "shield", "potion"];
let drop = loot.choose() ?? "nothing";
```

### `contains`

```mimas
fn contains(self, value: T) -> bool
```

Returns whether any element equals `value`. This is the same check as the [`in` operator](../reference/collections/in-expressions.md).

```mimas
let xs = [1, 2, 3];
let a = xs.contains(2); // true
let b = 2 in xs;        // true
```

### `enumerate`

```mimas
fn enumerate(self) -> [(int, T)]
```

Returns a new array pairing each element with its index.

```mimas
for (i, name) in ["ant", "bee"].enumerate() {
    print(f"{i}: {name}"); // 0: ant, then 1: bee
}
```

### `extend`

```mimas
fn extend(self, other: [T])
```

Appends every element of `other`, in order. `other` is unchanged.

```mimas
let xs = [1, 2];
xs.extend([3, 4]); // xs is now [1, 2, 3, 4]
```

### `flatten`

```mimas
fn flatten(self: [[T]]) -> [T]
```

Returns a new array with the elements of each inner array, in order. Only one level is removed: a `[[[int]]]` flattens to a `[[int]]`.

```mimas
let xs = [[1, 2], [], [3]].flatten(); // [1, 2, 3]
```

### `is_empty`

```mimas
fn is_empty(self) -> bool
```

Returns whether the array has no elements.

```mimas
let xs: [int] = [];
let empty = xs.is_empty(); // true
```

### `join`

```mimas
fn join(self: [str], separator: str) -> str
```

Returns the strings joined together with `separator` between each one. Only `[str]` has `join`, so convert other arrays first:

```mimas
let a = ["a", "b", "c"].join(", "); // "a, b, c"
let b = (for n in [1, 2, 3] collect n.to_str()).join("-"); // "1-2-3"
```

### `len`

```mimas
fn len(self) -> int
```

Returns the number of elements.

```mimas
let n = [3, 1, 2].len(); // 3
```

### `max`

```mimas
fn max(self: [int]) -> int?
fn max(self: [float]) -> float?
```

Returns the largest element, or `null` if the array is empty.

```mimas
let a = [3, 9, 4].max(); // 9
let b: [int] = [];
let c = b.max() ?? 0;    // 0
```

### `min`

```mimas
fn min(self: [int]) -> int?
fn min(self: [float]) -> float?
```

Returns the smallest element, or `null` if the array is empty.

```mimas
let a = [3, 9, 4].min(); // 3
```

### `pop`

```mimas
fn pop(self) -> T?
```

Removes the last element and returns it, or returns `null` if the array is empty.

```mimas
let xs = [1, 2];
let last = xs.pop(); // 2, and xs is now [1]
```

### `push`

```mimas
fn push(self, value: T)
```

Appends `value` to the end of the array.

```mimas
let xs = [1, 2];
xs.push(3); // xs is now [1, 2, 3]
```

### `reorder`

```mimas
fn reorder(self, indices: [int])
```

Replaces the array's contents with the elements at `indices`, in that order. The indices don't have to cover every element once: the array ends up as long as `indices`, and an index can repeat.

An index that is negative or past the end is a runtime error, and the array is left unchanged.

```mimas
let xs = ["a", "b", "c"];
xs.reorder([2, 0, 1]); // xs is now ["c", "a", "b"]
xs.reorder([0, 0]);    // xs is now ["c", "c"]
```

### `shuffle`

```mimas
fn shuffle(self)
```

Puts the elements in a random order.

```mimas
let deck = [1, 2, 3, 4];
deck.shuffle();
```

### `sort_by_float`

```mimas
fn sort_by_float(self, keys: [float])
```

Sorts the array in ascending order of `keys`, where `keys[i]` is the key for element `i`. Equal keys keep their original order. `keys` is unchanged.

```mimas
let names = ["far", "near", "mid"];
names.sort_by_float([9.5, 0.5, 3.0]); // names is now ["near", "mid", "far"]
```

```admonish warning title="Keys must match the array's length"
An element without a key is dropped from the array, and extra keys are ignored.
```

### `sort_by_int`

```mimas
fn sort_by_int(self, keys: [int])
```

Sorts the array in ascending order of `keys`, where `keys[i]` is the key for element `i`. Equal keys keep their original order. `keys` is unchanged.

```mimas
let names = ["cat", "ant", "bee"];
names.sort_by_int([3, 1, 2]); // names is now ["ant", "bee", "cat"]
```

```admonish warning title="Keys must match the array's length"
An element without a key is dropped from the array, and extra keys are ignored.
```

### `sum`

```mimas
fn sum(self: [int]) -> int
fn sum(self: [float]) -> float
```

Returns the elements added together, or `0` if the array is empty.

```mimas
let a = [1, 2, 3].sum();  // 6
let b = [0.5, 1.5].sum(); // 2.0
```
