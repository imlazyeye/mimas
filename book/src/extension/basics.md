# Basics

Providing new types and functions to mimas is made easy by the `mimas` macro.

```rust
// rust
#[mimas]
const HELLO: &str = "Hello!";

#[mimas]
fn echo(message: &str) {
    println!("{message}");
}

#[mimas]
struct MyType(usize);
```

By default the macro will place its imported items into the prelude, meaning your mimas code can reach it from anywhere.

```mimas
// mimas
let hello: str = HELLO;
```

Alternatively, you can dynamically create modules by specifying them to the macro.

```rs
// rust
#[mimas(entities)]
const ENTITY_COUNT: usize = 2;

#[mimas(entities::enemies)]
struct Monster {
    health: usize,
}
```

```mimas
// mimas
print(entities::ENTITY_COUNT);

let monster = entities::enemies::Monster {
    health = 0,
};
```

Methods come along too: tag an `impl` block and everything inside attaches to the type. `self`, `&self`, and `&mut self` all work, as do associated functions like constructors and constants. Methods belong to their type rather than a module, so the `impl` form takes no module path.

```rs
// rust
#[mimas]
struct Player {
    name: String,
    health: i64,
}

#[mimas]
impl Player {
    const MAX_HEALTH: i64 = 100;

    fn new(name: String) -> Self {
        Player { name, health: Self::MAX_HEALTH }
    }

    fn damage(&mut self, amount: i64) {
        self.health -= amount;
    }
}
```

```mimas
// mimas
let player = Player::new("Ferris");
player.damage(30);
print(player.health); // 70
```

```admonish info title="Write-back mutation"
Your `Player` doesn't live inside the VM -- its fields do. Calling a method copies the fields out into a real `Player`, runs your Rust, and (for `&mut self`) writes the fields back into the same instance when it returns.
```

````admonish warning title="Magic is dangerous!"
We use the [inventory](https://github.com/dtolnay/inventory) crate to automatically make your `mimas` tagged items visible to mimas. For most projects (where their API will be defined in the same crate that runs mimas) this carries no complications. However, if you ever wish for your API declarations to _cross crates_, you need to do one of the following:

1. Pin the crate holding them to a single codegen unit in your root `Cargo.toml`, and make sure your binary actually uses that crate (calling any one function from it is enough):

   ```toml
   [profile.dev.package.my-api]
   codegen-units = 1
   [profile.release.package.my-api]
   codegen-units = 1
   ```

2. Skip the magic for those items and register them by hand: write them with `#[native]` and add them through an installer function. We go over this in depth on the [Advanced Usage](./advanced-usage.md) page.

Without one of these, the linker is free to drop library code it sees no direct use for, and your items can silently vanish -- sometimes only in certain builds.
````