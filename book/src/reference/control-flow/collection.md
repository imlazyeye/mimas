# Collect

`collect` is a control-flow operator unique to mimas. It turns any loop into an array builder: each value you `collect` is appended to a running list, and when the loop finishes -- whether it runs out of iterations or hits a `break` -- the whole loop evaluates to that array.

Think of it as `continue` that carries a value: like `continue`, it moves on to the next iteration.

```mimas
let numbers = [1, 5, 9, 2];

let doubled = for x in numbers collect x * 2;
// -> [2, 10, 18, 4]
```

Loop bodies don't have to be blocks, so that last example fits on one line -- close to a Python list comprehension:

```mimas
// python:  doubled = [x * 2 for x in numbers]
let doubled = for x in numbers collect x * 2;
```

Add an `if` to filter which values get collected:

```mimas
let evens = for x in numbers {
    if x % 2 == 0 {
        collect x;
    }
};
// -> [2]
```

It composes with every loop, including [`while let`](./while.md#while-let), which is handy for gathering results from a source until it's exhausted:

```mimas
let ages: [int] = while let person = next_person() {
    collect person.age;
};
```

```admonish info title="'collect' and 'break value' don't mix"
A loop either *builds* a value with `collect` or *breaks out with* one, not both. Doing both in the same loop is a compile error, since the loop can't be an array and a single value at once.
```
