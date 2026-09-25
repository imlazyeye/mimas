# Scripts

There is no implicit `main` function in mimas. Unlike Rust, a script just runs top to bottom -- write whatever code you like at the file's root and execute it directly, the way you would with Python or JavaScript.

```mimas
// hello.mim
print("Hello, world!"); // runs the moment you execute the file
```

```sh
mimas run hello.mim
```

Top-level statements run in order, and top-level `fn`s, `struct`s, and similar declarations are available throughout the file regardless of where they're declared.

```mimas
print(greet("world"));      // works -- `greet` is visible across the whole file

fn greet(name: str) -> str {
    f"hello, {name}!"
}
```

```admonish note title="Scripts run; modules organize"
A file that opens with a [`module`](./modules.md) declaration is a library, not a script -- it can't carry top-level expressions to execute. This split keeps "the thing you run" distinct from "the code you structure for reuse."
```

## Projects

A project is a directory. Its scripts and modules can sit anywhere in it, subdirectories included, without any setup. Another cargo package or a cargo `target` directory inside it isn't part of it. `mimas run game.mim` checks `game.mim` together with every module in its directory and the directories under it, and `mimas check my_project` does the same for each script in `my_project`, one at a time. Scripts never see each other, so two scripts can both declare `fn update` without clashing.

`mimas run` on a directory runs its only script. When the directory holds more than one, name the one you mean.

```sh
mimas run my_project           # runs the one script in my_project
mimas run my_project/game.mim  # or picks one of many
```
