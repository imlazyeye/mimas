# The `in` Operator

`in` tests for membership and evaluates to a `bool`. It works across the collection types and strings, adapting its meaning to each:

| `x in y` where `y` is... | tests whether... |
| :--- | :--- |
| `[T]` (array) | the array contains the element `x` |
| `~{V}` (dict) | the dict has the **key** `x` (a `str`) |
| `str` | `x` is a substring of `y` |

```mimas
let nums = [0, 1, 2];
print(0 in nums);        // true

let dict = ~{ foo = 1 };
print("foo" in dict);    // true  -- checks keys

let text = "hello!";
print("ell" in text);    // true  -- substring
print("x" in text);      // false
```

For an array, `x in xs` is the operator form of `xs.contains(x)`; use whichever reads better at the call site.
