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
