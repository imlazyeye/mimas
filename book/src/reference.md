# Language Reference

This is the complete reference for the mimas language in version `0.1.0`. It describes the syntax and mechanics of the language itself -- the parts you write in a `.mim` file. Extending mimas from Rust is covered separately in [Extension with Rust](./extension-with-rust.md).

Each page introduces one concept with short examples. Most are complete programs you can drop into a file and run; a few are fragments or deliberate errors, noted where they appear:

```sh
mimas example.mim
```

| Page | Covers |
| :-- | :-- |
| [Variables & Constants](./reference/variables.md) | `let`, `const`, and shadowing |
| [Primitive Types](./reference/basic-types.md) | `int`, `float`, `str`, and `bool` |
| [Operators](./reference/operations.md) | Arithmetic, comparison, logic, and bit operations |
| [Blocks & Scope](./reference/blocks.md) | Blocks are expressions; what a scope can see |
| [Unit & Never Types](./reference/special-types.md) | `()`, `!`, and what "no value" means |
| [Control Flow](./reference/control-flow.md) | `if` and `if let`, `match`, the three loops, and `collect` |
| [Collections](./reference/collections.md) | Arrays, dictionaries, tuples, and the `in` operator |
| [Options](./reference/options.md) | `T?`, `null`, and the operators that handle them |
| [Results & Error Handling](./reference/error-handling.md) | `T!`, `raise`, and `absolve` |
| [Functions & Closures](./reference/functions.md) | Declarations, defaults and named arguments, closures |
| [User-Defined Types](./reference/types.md) | Structs, enums, and `impl` blocks |
| [Pacts](./reference/pacts.md) | Abstracting over types without generics |
| [Privacy](./reference/privacy.md) | `pub` and the module boundary |
| [Modules](./reference/modules.md) | Declaring, nesting, and importing |
| [Scripts](./reference/scripts.md) | No `main`; files run top to bottom |
| [Memory](./reference/memory.md) | Garbage collection and its limits |
| [Notable Exclusions](./reference/notable-exclusions.md) | What mimas leaves out, and why |
