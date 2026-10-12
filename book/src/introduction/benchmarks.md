# Benchmarks

The core ethos of mimas is **flexibility with safety**. The upshot of the latter is a plethora of guarantees for the compiler and runtime, allowing it to optimize the bytecode very tightly.

The [home page](../introduction.md) measures mimas against the other pure-Rust scripting runtimes. Here we'll narrow down with the intention of showing the most likely alternative options one would use against mimas.

[Rune](https://rune-rs.github.io/) and [Rhai](https://rhai.rs/) are the pure-Rust points of comparison: Rune is the quickest of them after mimas, and Rhai is feature-rich and by far the most widely used in the space. [Luau](https://luau.org/) (through [mlua](https://github.com/mlua-rs/mlua)) is is the quickest thing you can embed in a Rust program, full stop. However, unlike the others, it is **written in C++.**

<!-- benchmark start -->
<div class="bench"><h4><a href="https://github.com/imlazyeye/mimas/tree/main/benchmarks/strings">strings</a> <small>string formatting, builtin methods</small></h4>
<table class="charts-css bar show-labels data-spacing-4"><tbody>
<tr><th scope="row">mimas</th><td class="me" style="--size:0.19999999999999998"><span class="data">538ms</span></td></tr>
<tr><th scope="row">luau</th><td style="--size:0.3278121946016272"><span class="data">881ms</span></td></tr>
<tr><th scope="row">rune</th><td class="overflow" style="--size:1"><span class="data">2.99s</span></td></tr>
<tr><th scope="row">rhai</th><td class="overflow" style="--size:1"><span class="data">4.50s</span></td></tr>
</tbody></table></div>
<div class="bench"><h4><a href="https://github.com/imlazyeye/mimas/tree/main/benchmarks/physics">physics</a> <small>struct field access, float math</small></h4>
<table class="charts-css bar show-labels data-spacing-4"><tbody>
<tr><th scope="row">mimas</th><td class="me" style="--size:0.19999999999999998"><span class="data">836ms</span></td></tr>
<tr><th scope="row">luau</th><td style="--size:0.28220845588950655"><span class="data">1.18s</span></td></tr>
<tr><th scope="row">rune</th><td style="--size:0.8396874251002358"><span class="data">3.51s</span></td></tr>
<tr><th scope="row">rhai</th><td class="overflow" style="--size:1"><span class="data">14.32s</span></td></tr>
</tbody></table></div>
<div class="bench"><h4><a href="https://github.com/imlazyeye/mimas/tree/main/benchmarks/mandelbrot">mandelbrot</a> <small>scalar float throughput, tight loops</small></h4>
<table class="charts-css bar show-labels data-spacing-4"><tbody>
<tr><th scope="row">luau</th><td style="--size:0.2"><span class="data">691ms</span></td></tr>
<tr><th scope="row">mimas</th><td class="me" style="--size:0.21618501551617755"><span class="data">747ms</span></td></tr>
<tr><th scope="row">rune</th><td style="--size:0.46780851976571025"><span class="data">1.62s</span></td></tr>
<tr><th scope="row">rhai</th><td class="overflow" style="--size:1"><span class="data">7.78s</span></td></tr>
</tbody></table></div>
<div class="bench"><h4><a href="https://github.com/imlazyeye/mimas/tree/main/benchmarks/prime_numbers">prime numbers</a> <small>array indexing, tight integer loops</small></h4>
<table class="charts-css bar show-labels data-spacing-4"><tbody>
<tr><th scope="row">mimas</th><td class="me" style="--size:0.2"><span class="data">568ms</span></td></tr>
<tr><th scope="row">luau</th><td style="--size:0.22515293550854393"><span class="data">639ms</span></td></tr>
<tr><th scope="row">rune</th><td class="overflow" style="--size:1"><span class="data">3.42s</span></td></tr>
<tr><th scope="row">rhai</th><td class="overflow" style="--size:1"><span class="data">6.27s</span></td></tr>
</tbody></table></div>
<div class="bench"><h4><a href="https://github.com/imlazyeye/mimas/tree/main/benchmarks/fibonacci">fibonacci</a> <small>function-call overhead, recursion</small></h4>
<table class="charts-css bar show-labels data-spacing-4"><tbody>
<tr><th scope="row">mimas</th><td class="me" style="--size:0.19999999999999998"><span class="data">711ms</span></td></tr>
<tr><th scope="row">luau</th><td style="--size:0.27147553879185776"><span class="data">965ms</span></td></tr>
<tr><th scope="row">rune</th><td style="--size:0.5788988878623109"><span class="data">2.06s</span></td></tr>
<tr><th scope="row">rhai</th><td class="overflow" style="--size:1"><span class="data">7.81s</span></td></tr>
</tbody></table></div>
<div class="bench"><h4><a href="https://github.com/imlazyeye/mimas/tree/main/benchmarks/eval">eval</a> <small>enum match dispatch, recursion</small></h4>
<table class="charts-css bar show-labels data-spacing-4"><tbody>
<tr><th scope="row">luau</th><td style="--size:0.2"><span class="data">652ms</span></td></tr>
<tr><th scope="row">mimas</th><td class="me" style="--size:0.27262752645569793"><span class="data">888ms</span></td></tr>
<tr><th scope="row">rune</th><td style="--size:0.6000602627224595"><span class="data">1.95s</span></td></tr>
<tr><th scope="row">rhai</th><td class="overflow" style="--size:1"><span class="data">63.05s</span></td></tr>
</tbody></table></div>
<div class="bench"><h4><a href="https://github.com/imlazyeye/mimas/tree/main/benchmarks/collections">collections</a> <small>dict insert + lookup, string keys</small></h4>
<table class="charts-css bar show-labels data-spacing-4"><tbody>
<tr><th scope="row">mimas</th><td class="me" style="--size:0.2"><span class="data">329ms</span></td></tr>
<tr><th scope="row">luau</th><td style="--size:0.2347601075698553"><span class="data">387ms</span></td></tr>
<tr><th scope="row">rune</th><td class="overflow" style="--size:1"><span class="data">2.81s</span></td></tr>
<tr><th scope="row">rhai</th><td class="overflow" style="--size:1"><span class="data">3.33s</span></td></tr>
</tbody></table></div>
<p class="bench-legend"><a href="https://github.com/imlazyeye/mimas">mimas</a> v0.4.0 · <a href="https://github.com/rune-rs/rune">Rune</a> v0.14.2 · <a href="https://github.com/rhaiscript/rhai">Rhai (perf)</a> v1.26.0 · <a href="https://github.com/luau-lang/luau">Luau (mlua)</a> v0.11.4 -- measured on a 2021 M1 Max MacBook Pro. Bars are scaled within each test. Results slower than 5x of the fastest result are faded and not factored into the scaling.</p>
<!-- benchmark end -->

````admonish info title="What these actually reflect"

What is idiomatic in one language very well may not match another, but these tests are written to be as structurally similar as possible -- the same approach taken by [*Are We Fast Yet?*](https://github.com/smarr/are-we-fast-yet), which compares languages on equivalent code rather than language-specific tricks. The intention is to measure how the same practices perform in each language. They therefore measure the *cost of the abstraction*, not raw arithmetic throughput: **read them as "what does modelling your data this way cost?", not "which language computes faster".**
````

```admonish faq title="Why stop at bytecode?"

The next step down from mimas's current bytecode VM would be JIT compilation using something like [Cranelift](https://cranelift.dev/). A JIT has to write fresh machine code into memory at runtime, and many of the platforms mimas cares about forbid exactly that. Game consoles (PlayStation, Xbox, Switch) disallow runtime-generated executable memory as a hard certification requirement; Apple's platforms (iOS and its siblings) disallow it for third-party apps; sandboxes like the browser don't offer it either.

[Pulley](https://docs.wasmtime.dev/examples-pulley.html) remains an option that could improve performance further, which will be later explored.
```

## Compile speed

The compiler is fast enough that you effectively won't notice any compile times, running at about **400,000 lines per second**. More timing information below.

| program | lines | compile |
| :---: | :---: | :---: |
| a small module | ~100 | < 1 ms |
| a project | ~10,000 | ~16 ms |
| a large project | ~100,000 | ~0.20 s |

<p class="bench-legend">Measured with `mimas build` on the same machine, over a representative collection of source (consts, structs, enums, functions, deeply-nested types) generated by the fodder project (`tools/fodder`).</p>

## Reproduce

You can calculate these tests on your own machine with `benchmarks/compare.sh` (which needs [hyperfine](https://github.com/sharkdp/hyperfine)), and the compile-speed table with `benchmarks/compile-bench.sh`. `cargo bench -p mimas` runs our own benchmarks through [criterion](https://github.com/bheisler/criterion.rs).
