# Operators

mimas's operators will look familiar coming from Rust or C-family languages. This page is the full list.

## Arithmetic

| Name | Symbol | Types | Behavior |
| :--- | :---: | :--- | :--- |
| Add | `+` | `int`, `float`, `str` | Adds two numbers, or concatenates two `str`s. |
| Subtract | `-` | `int`, `float` | Subtracts. |
| Multiply | `*` | `int`, `float` | Multiplies. |
| Divide | `/` | `int`, `float` | Divides. **Always yields a `float`.** |
| Truncating divide | `~/` | `int`, `float` | Divides and truncates toward zero, keeping the operand type. |
| Modulo | `%` | `int`, `float` | Remainder after division (sign follows the left operand). |

```mimas
let a = 7 / 2;   // 3.5  (float)
let b = 7 ~/ 2;  // 3    (int)
let c = 7 % 2;   // 1
let d = "ab" + "cd"; // "abcd"
```

```admonish warning title="int + float promotes; / always floats"
Mixing an `int` and a `float` promotes the result to `float`. And `/` produces a `float` even between two `int`s -- use `~/` for integer division. Integer arithmetic is checked, so overflow is a runtime error, never a silent wrap.
```

## Comparison

Comparisons produce a `bool`. Ordering (`<`, `<=`, `>`, `>=`) is defined for numbers only; `str` supports equality but not ordering.

| Name | Symbol | Types |
| :--- | :---: | :--- |
| Equal | `==` | any matching pair |
| Not equal | `!=` | any matching pair |
| Less / less-or-equal | `<` `<=` | `int`, `float` |
| Greater / greater-or-equal | `>` `>=` | `int`, `float` |

```mimas
print(2 < 3);          // true
print("hi" == "hi");   // true
print("a" < "b");      // compile error: these values cannot be compared like numerals
```

## Logical

| Name | Symbol | Behavior |
| :--- | :---: | :--- |
| And | `&&` | `true` if both operands are true. Short-circuits. |
| Or | `\|\|` | `true` if either operand is true. Short-circuits. |
| Not | `!` (prefix) | Negates a `bool`. |

```mimas
let ok = is_ready() && !is_locked();
```

## Bitwise

Bitwise operators work on `int`.

| Name | Symbol | Behavior |
| :--- | :---: | :--- |
| And | `&` | Set each bit where both bits are set. |
| Or | `\|` | Set each bit where either bit is set. |
| Xor | `^` | Set each bit where the bits differ. |
| Shift left | `<<` | Shift bits left. |
| Shift right | `>>` | Shift bits right. |

```mimas
print(6 & 3);  // 2
print(6 | 1);  // 7
print(6 ^ 3);  // 5
print(1 << 4); // 16
```

## Null coalescing

The `??` operator supplies a fallback when its left operand is `null`. It is short-circuiting -- the right side is only evaluated when needed -- and it belongs to the [option](./options.md) family of operators.

```mimas
let name: str? = null;
let shown = name ?? "anonymous"; // -> "anonymous"
```

## Assignment

Every arithmetic, bitwise, and coalescing operator has a compound-assignment form that updates a binding in place.

```mimas
a += 1;
a -= 1;
a *= 2;
a /= 2;
a %= 3;
a ~/= 2;
a &= 1;
a |= 1;
a ^= 1;
a ??= fallback; // assign only if `a` is currently null
```
