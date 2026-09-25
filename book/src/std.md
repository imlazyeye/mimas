# Standard Library

This is the reference for everything mimas ships with. It lists each function, method, and constant, what it takes and returns, and how it behaves. For the language itself, see the [Language Reference](./reference.md).

The standard library comes in three parts:

- **The prelude**: functions like `print` and `panic` that are in scope in every script.
- **Methods on the built-in types**: `len` on arrays, `to_upper` on strings, and so on. These are always available through dot syntax.
- **Modules under `std`**: everything else, like file access and JSON parsing. Bring these in with `use`, such as `use std::fs;`.

| Page | Covers |
| :-- | :-- |
