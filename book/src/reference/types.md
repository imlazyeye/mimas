# User-Defined Types

Beyond the built-in primitives and collections, mimas lets you define your own named types. There are two kinds, the algebraic product/sum pair:

- **[Structs](./types/structs.md)** -- product types. A struct holds several values at once: a `Player` has a name, a score, and a position.
- **[Enums](./types/enums.md)** -- sum types. An enum is exactly one of several shapes: a `Shape` is a circle, a rectangle, or a triangle, never more than one at a time.

Both gain behavior through `impl` blocks, and both can satisfy a [pact](./pacts.md) to share an interface.
