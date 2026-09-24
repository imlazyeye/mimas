# Standard Library

This is the reference for everything mimas ships with, in version `0.1.0`. It lists each function, method, and constant, what it takes and returns, and how it behaves. For the language itself, see the [Language Reference](./reference.md).

The standard library comes in three parts:

- **The prelude**: functions like `print` and `panic` that are in scope in every script.
- **Methods on the built-in types**: `len` on arrays, `to_upper` on strings, and so on. These are always available through dot syntax.
- **Modules under `std`**: everything else, like file access and JSON parsing. Bring these in with `use`, such as `use std::fs;`.

| Page | Covers |
| :-- | :-- |
| Prelude | `print`, `panic`, `todo`, and `dbg` |
| [Arrays](./std/arrays.md) | Growing, searching, and sorting `[T]` |
| Dictionaries | Inserting, removing, and iterating `~{V}` |
| Strings | Searching, splitting, and converting `str` |
| Integers | Math and conversions on `int` |
| Floats | Rounding, trigonometry, and conversions on `float` |
| Booleans | Methods on `bool` |
| `std::fs` | Reading, writing, and walking files and directories |
| `std::math` | Constants and vector types |
| `std::parse` | JSON in and out |
| `std::process` | Running other programs |
| `std::sys` | Arguments, stdin, and exiting |
