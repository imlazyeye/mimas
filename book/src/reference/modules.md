# Modules

A module is a named collection of items -- functions, constants, types -- that you can share across files. Modules are how a mimas project grows past a single script: they group related code and draw the [privacy](./privacy.md) boundary that `pub` controls.

## Declaring a module

A file becomes a module with a `module` declaration at the top. Name it explicitly, or use `@` to take the file's own name (so `colors.mim` becomes the `colors` module).

```mimas
module colors;
// equivalently, in colors.mim:
module @;

const INTERNAL = 0; // private to this module
pub const RED = "#ff0000";
pub fn mix(a: str, b: str) -> str { /* ... */ }
```

Modules can nest by qualifying the path with `::`:

```mimas
module graphics::colors;
```

```admonish note title="Module files don't run"
A file that declares a module is a *library*, not a *script* -- it can't have top-level expressions to execute. That keeps "code that runs" and "code that's organized for reuse" separate. See [Scripts](./scripts.md).
```

## Using a module

Bring a module into scope with `use`. By default this makes the module available under its name, and you reach its public items with `::` -- the same accessor structs use.

```mimas
use colors;

let r = colors::RED;
let m = colors::mix(colors::RED, "#00ff00");

let i = colors::INTERNAL; // error: `INTERNAL` is private to its module
```

You can also pull items *directly* into the current file. Import one item, several at once with braces, or everything with `*`:

```mimas
use colors::RED;          // just `RED`
use colors::{ RED, mix }; // several names
use colors::*;            // every public item

let r = RED;
let m = mix(RED, "#00ff00");
```
