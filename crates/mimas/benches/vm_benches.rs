use std::{hint::black_box, time::Duration};

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use mimas::{library, vm::Vm};

const PROGRAMS: &[(&str, &str)] = &[
    (
        "mandelbrot",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../benchmarks/mandelbrot/mandelbrot.mim"
        )),
    ),
    (
        "fibonacci",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../benchmarks/fibonacci/fibonacci.mim"
        )),
    ),
    (
        "strings",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../benchmarks/strings/strings.mim"
        )),
    ),
    (
        "prime",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../benchmarks/prime_numbers/prime_numbers.mim"
        )),
    ),
    (
        "physics",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../benchmarks/physics/physics.mim"
        )),
    ),
];

// parse + solve + lower + codegen, no execution.
fn compile(c: &mut Criterion) {
    let mut g = c.benchmark_group("compile");
    for &(name, src) in PROGRAMS {
        let src = strip_print(src);
        g.bench_function(name, |b| {
            b.iter(|| Vm::compile(black_box(src.as_str()), library::std).unwrap())
        });
    }
    g.finish();
}

// execution only -- compilation happens in the untimed `iter_batched` setup, and the Vm is
// returned from the routine so arena teardown is dropped untimed.
fn run(c: &mut Criterion) {
    let mut g = c.benchmark_group("run");
    for &(name, src) in PROGRAMS {
        let src = strip_print(src);
        g.bench_function(name, |b| {
            b.iter_batched(
                || Vm::compile(src.as_str(), library::std).unwrap(),
                |mut vm| {
                    vm.run().unwrap();
                    vm
                },
                BatchSize::PerIteration,
            )
        });
    }
    g.finish();
}

// the bench files carry a trailing `print(...)` so the cross-language compare can't dead-code the
// result; drop it here so it doesn't run inside the timed region or flood bench output.
fn strip_print(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("print("))
        .collect::<Vec<_>>()
        .join("\n")
}

criterion_group! { name = compile_g; config = Criterion::default(); targets = compile }
criterion_group! {
    name = run_g;
    config = Criterion::default()
        .sample_size(20)
        .without_plots()
        .measurement_time(Duration::from_secs(30))
        .warm_up_time(Duration::from_secs(2));
    targets = run
}

criterion_main!(compile_g, run_g);
