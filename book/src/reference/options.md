# Options

When a value is allowed to be either some type *or* `null`, it's an **option**. Mark a type as optional by suffixing it with `?`.

```mimas
let a: int = 0;
a = null; // compile error: `a` is int, never null

let b: int? = 0;
b = null; // valid
```

An option is mimas's answer to the [billion-dollar mistake](https://www.infoq.com/presentations/Null-References-The-Billion-Dollar-Mistake-Tony-Hoare/): `null` only exists where a `?` invites it, so the compiler can guarantee you never trip over an unexpected null access.

## An option is not its inner type

`int?` and `int` are different types. You can't use a possibly-null value where a definitely-present one is required -- you have to deal with the `null` case first ([unwrapping](#unwrapping), below).

```mimas
let a: int = 0;
let b: int? = null;
a = b; // compile error: `b` may be null
```

## Comparing options

You can compare an option for equality against `null`, or against a value of its inner type:

```mimas
let maybe: int? = null;

print(maybe == null); // true
print(maybe == 0);    // false
print(0 == maybe);    // false -- order doesn't matter
```

Ordering comparisons (`<`, `>`, and so on), on the other hand, need both sides to be non-null -- there's no sensible place for `null` on a number line. Unwrap or [coalesce](#unwrapping) first.

```mimas
let a: int = 0;
let b: int? = null;
print(a > b); // compile error: these values can't be compared like numerals
```

## Creating options

Any type coerces *into* its option form when it meets a `null`. So an array literal with a `null` in it, or an `if` whose branches disagree about presence, infers an optional type automatically.

```mimas
let a = [0, null, 1];               // a is [int?]
let b = ~{ x = null, y = 0 };       // b is ~{int?}
let c = if ready { 0 } else { null }; // c is int?
```

The coercion only kicks in when a fresh value is created. It won't quietly punch a `null` into an existing non-optional slot:

```mimas
let d = [0];
d[0] = null; // compile error: expected int, found null
```

## Flattening

There is no "option of an option." If a type would come out as `T??`, mimas automatically flattens it to `T?` -- the same choice Kotlin makes.

```mimas
fn maybe() -> int? { null }

// `maybe()` is already int?, and the other branch is null, so this would be int??
// -- mimas flattens it straight to int?.
let nested: int? = if ready { maybe() } else { null };
```

## Option chaining

To reach through a value that might be `null`, you'd otherwise write a guard:

```mimas
let len: int? = if name == null {
    null
} else {
    name.len()
};
```

The `?.` operator collapses that into one expression. If the receiver is `null`, the whole chain short-circuits to `null` and the call is never made; otherwise it proceeds and re-wraps the result as an option.

```mimas
let len: int? = name?.len();
```

The same works for indexing, with `?[ ]`:

```mimas
let grid: [[int]?] = [[0], null, [1]];
let cell: int? = grid[1]?[0]; // null -- the middle row is null, so we stop
```

### Chaining past the first optional

A `?` short-circuits past *one* option. Drilling further with a plain `[i]` or `.field` rides that same short-circuit -- those don't introduce a new `null` -- so you don't repeat `?` to reach into an array or a struct field:

```mimas
let grid: [[int]]? = null;

// one `?` for `grid`; the `[0]` and `[1]` array drills ride it.
let cell: int? = grid?[0][1]; // null if `grid` is null
```

A dict index is different: the key might miss, so each dict you reach *through* spends its own `?`. With a `~{~{[int]}}`, both dict hops are options:

```mimas
let f: ~{~{[int]}} = ~{ a = ~{ b = [0] } };

let x: int? = f["a"]?["b"]?[0]; // two dicts -> two `?`, then the array drill rides
// f["a"]?["b"][0]  // compile error: the `["b"]` miss isn't handled before `[0]`
```

The rule is one `?` for each option you reach *through*. A trailing option is fine -- you spend a `?` to index past one, not to produce one:

```mimas
let users: ~{~{int}} = ~{ alice = ~{ score = 10 } };
let score: int? = users["alice"]?["score"]; // `?` reaches through ["alice"]; the result is the option
```

A chain only rides a real `?` token. A bare dict index that happens to be an option does not silently propagate -- `users["alice"]["score"]` is a compile error, because the middle value may be null and you haven't said how to handle it. Parentheses close a chain too, so `(grid?[0])[1]` stops at the paren and the outer `[1]` sees a plain option again.

## Unwrapping

When a code path expects an option to actually hold a value, you **unwrap** it. Where `?` poses the *question* of an option, a postfix `!` asserts the *answer*: "this is not null." If it turns out to be `null`, that's a runtime error.

```mimas
let port: int = lookup_port()!; // `port` is a plain int from here on
```

When you'd rather supply a fallback than risk a runtime error, reach for `??`, which yields its right side when the left is `null`:

```mimas
let port: int = lookup_port() ?? 8080; // the value if present, otherwise 8080
```

And to branch on presence while binding the unwrapped value, use [`if let`](./control-flow/if-else.md#if-let):

```mimas
if let port = lookup_port() {
    connect(port);
} else {
    use_default();
}
```

When you want the unwrapped value for the rest of the scope and would rather bail out than nest, a [`let`/`else`](./variables.md#let-else) keeps things flat:

```mimas
let port? = lookup_port() else {
    panic("no port configured");
};
// `port` is a plain int from here on
```
