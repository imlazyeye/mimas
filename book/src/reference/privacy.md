# Privacy

Privacy in mimas is drawn at the **[module](./modules.md) boundary**. Everything is private to its own module by default; the `pub` keyword is what makes an item -- or a struct field -- reachable from *other* modules.

```mimas
pub fn api() {}      // callable from other modules
fn helper() {}       // module-private

pub struct Config {
    pub name: str,   // readable from other modules
    secret: str,     // module-private field
}
```

```admonish note title="Within a module, there are no walls"
`pub` only matters when one module reaches into another. Inside a single module (including a plain script file), every item and field is freely accessible. The examples below assume the access is happening from a *different* module.
```

## Private fields

From outside its module, a struct's private fields are neither readable nor writable. That also means you can't build the struct with a literal -- you'd have to name fields you aren't allowed to touch.

```mimas
// in another module:
let c = Config { name = "x", secret = "y" }; // error: `Config::secret` is not accessible
let s = c.secret;                            // error: `Config::secret` is not accessible
```

The fix is to expose a public surface -- a constructor and accessors -- and keep the internals sealed:

```mimas
impl Config {
    pub fn new(name: str) -> Self {
        Self { name = name, secret = generate() }
    }

    pub fn name(self) -> str {
        self.name
    }
}

// in another module:
let c = Config::new("x"); // ok
print(c.name());          // ok
```

This is the standard encapsulation move: callers get a stable, intentional interface, and you stay free to change the private guts.
