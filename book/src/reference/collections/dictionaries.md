# Dictionaries

A dictionary is a hash map from **string keys** to values of one type, written `~{V}`. The leading `~` distinguishes a dict literal from a [block](../blocks.md), since both use braces.

In a literal, keys are written as bare identifiers and values follow an `=`. You read a value back with `["key"]`.

```mimas
let scores: ~{int} = ~{
    alice = 10,
    bob = 7,
};
let top: int? = scores["alice"]; // 10
```

```admonish note title="Keys are strings"
The identifier syntax in a literal (`alice = 10`) is sugar -- the key is the string `"alice"`, and you index with a string (`scores["alice"]`). Dictionaries are plain hash maps, not an ADT; for fixed, named fields with mixed types, use a [struct](../types/structs.md).
```

## Indexing returns an option

Any key might be absent, so indexing a dict hands back an [option](../options.md): `~{V}` yields `V?`, never a bare `V`. A present key gives the value, a missing one gives `null`, and you handle that `null` like any other option.

```mimas
let scores: ~{int} = ~{ alice = 10 };

let a: int? = scores["alice"]; // 10
let b: int? = scores["zoe"];   // null

let shown = b ?? 0; // 0 -- fall back when absent
```

To reach through a nested dict, each dict hop you index past takes its own `?` -- see [option chaining](../options.md#chaining-past-the-first-optional).

```mimas
let users: ~{~{int}} = ~{ alice = ~{ score = 10 } };
let score: int? = users["alice"]?["score"]; // 10, or null if either key is absent
```

## Iterating

Iterating a dict binds each entry as a `(key, value)` pair:

```mimas
for pair in ~{ x = 1, y = 2 } {
    print(pair); // [x, 1], then [y, 2]
}
```

Dictionaries also carry built-in methods for membership and mutation, such as `contains_key`, `insert`, and `remove`.
