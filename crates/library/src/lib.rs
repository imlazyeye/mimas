//! `library::std` is the standard-library install closure passed to [`vm::Vm::install_library`].
//! It registers everything explicitly (the `#[native]` fns + `#[derive(MimasEnum)]` adts) so the
//! std lib reaches every host with zero build config -- `library::std` directly names each item, so
//! they're always linked. (Authoring via `#[mimas]` would defer registration through `inventory`,
//! which only collects items whose object file is linked; for a dependency rlib like this, that
//! isn't guaranteed without the host setting `codegen-units = 1`. We eat the explicit boilerplate
//! here so downstream doesn't have to, because we love you. `#[mimas]` + the VM's install-time
//! sweep stay available for a *host's own* natives, which are reliably linked when defined in their
//! binary.)
mod methods {
    pub(crate) mod array;
    pub(crate) mod bool;
    pub(crate) mod dict;
    pub(crate) mod float;
    pub(crate) mod int;
    pub(crate) mod str;
}

mod std_lib {
    pub(crate) mod fs;
    pub(crate) mod math;
    pub(crate) mod parse;
    pub(crate) mod process;
    pub(crate) mod sys;
}

mod prelude;

pub use std_lib::sys::ScriptArgs;

use vm::api::Api;

/// Single entry point passed to `Vm::install_library`. Each `install` call registers its module's
/// natives explicitly (into both the solver metadata and the arena callable table).
pub fn std<'gc>(api: &mut Api<'_, 'gc>) {
    prelude::install(api);
    methods::array::install(api);
    methods::dict::install(api);
    methods::float::install(api);
    methods::int::install(api);
    methods::str::install(api);
    methods::bool::install(api);
    std_lib::fs::install(api);
    std_lib::math::install(api);
    std_lib::parse::install(api);
    std_lib::process::install(api);
    std_lib::sys::install(api);
}

#[cfg(test)]
mod tests {
    // `#[mimas]` items local to this test binary -- mirrors how a downstream *binary* registers its
    // own natives, and exercises the VM's install-time inventory sweep for each item kind. binary-
    // local submissions are always linked, so this is reliable regardless of codegen-units.
    #[vm::mimas(testing)]
    fn answer() -> i64 {
        42
    }

    #[vm::mimas(testing)]
    const ANSWER: i64 = 42;

    #[vm::mimas]
    struct Point(i64, i64);

    #[test]
    fn std_and_host_natives_resolve() {
        let snippets = [
            "print(\"x\");",                    // explicit prelude
            "use std::fs; fs::cwd();",          // explicit std module fn
            "std::parse::from_json(\"null\");", // explicit std fn + Value adt
            "use std::math::PI; print(PI);",    // explicit std const
            "print(testing::answer());",        // host #[mimas] fn, collected via the VM sweep
            "print(testing::ANSWER);",          // host #[mimas] const, collected via the VM sweep
            "print(Point(1, 2));",              // host #[mimas] struct adt, constructed in mimas
        ];
        for src in snippets {
            vm::Vm::compile(src, crate::std)
                .unwrap_or_else(|e| panic!("native failed to resolve for {src:?}: {e:?}"));
        }
    }

    // a `&mut Vec<anon::T>` native: a type-safe generic push where the pushed value must match the
    // array's element type (unlike `&mut Vec<Val>`, which is permissive). `anon::T` borrows the gc
    // storage mutably with no copy because it's a transparent `Val`.
    #[vm::native]
    fn push_typed<'gc>(ctx: vm::Ctx<'gc>, arr: &mut Vec<vm::anon::T<'gc>>, val: vm::anon::T<'gc>) {
        arr.push(val);
    }

    fn install_push_typed<'gc>(api: &mut vm::api::Api<'_, 'gc>) {
        crate::std(api);
        api.add_method(push_typed);
    }

    // read-only anon borrows are zero-copy AND keep their slot: `T`/`U` stay independent, and an
    // explicit `Anon<'gc, N>` links a borrow-shape param to a plain param through the same slot.
    #[vm::native]
    fn first_pair<'gc>(
        a: &[vm::anon::T<'gc>],
        b: &[vm::anon::U<'gc>],
    ) -> (vm::anon::T<'gc>, vm::anon::U<'gc>) {
        (a[0], b[0])
    }

    #[vm::native]
    fn pick<'gc>(
        a: &[vm::anon::Anon<'gc, 5>],
        fallback: vm::anon::Anon<'gc, 5>,
    ) -> vm::anon::Anon<'gc, 5> {
        if a.is_empty() { fallback } else { a[0] }
    }

    fn install_anon_natives<'gc>(api: &mut vm::api::Api<'_, 'gc>) {
        crate::std(api);
        api.add_method(first_pair);
        api.add_method(pick);
    }

    #[test]
    fn anon_slots_survive_borrow_shapes() {
        vm::Vm::execute(
            "let p = [1, 2].first_pair([\"a\"]); print(p.0 + 1); print(p.1.len());",
            install_anon_natives,
        )
        .expect("T and U element types should stay independent");
        vm::Vm::execute(
            "let a: [int] = []; print(a.pick(9) + 1);",
            install_anon_natives,
        )
        .expect("explicit Anon<'gc, 5> should register and link");
        assert!(
            vm::Vm::compile("let _ = [1].pick(\"x\");", install_anon_natives).is_err(),
            "fallback must link to the array's element type through slot 5",
        );
    }

    #[test]
    fn mut_anon_array_native() {
        // value matches the element type -> mutates in place and runs.
        vm::Vm::execute(
            "let a = [1, 2]; a.push_typed(3); print(a.len());",
            install_push_typed,
        )
        .expect("type-safe push of a matching element should run");
        // value mismatches the element type -> type error (the anon::T linkage enforces it).
        assert!(
            vm::Vm::compile("let a = [1]; a.push_typed(\"x\");", install_push_typed).is_err(),
            "pushing a str into an int array should be a type error",
        );
    }

    // the full host-state pattern: a FreezeCell fixture lends `&mut World` to natives for the
    // duration of one run (mirrors mistria's World-through-ctx wiring)
    mod world_fixture {
        use vm::{
            Ctx,
            freeze::{Freeze, FreezeCell},
        };

        struct World {
            score: i64,
        }

        type WorldCell = FreezeCell<Freeze![&'freeze mut World]>;

        trait WorldCtx<'gc> {
            fn world_mut<R>(self, f: impl FnOnce(&mut World) -> R) -> R;
        }
        impl<'gc> WorldCtx<'gc> for Ctx<'gc> {
            fn world_mut<R>(self, f: impl FnOnce(&mut World) -> R) -> R {
                self.fixture::<WorldCell>()
                    .with_mut(|w| f(w))
                    .expect("World accessed outside a freeze scope")
            }
        }

        #[vm::native]
        fn add_score<'gc>(ctx: Ctx<'gc>, n: i64) -> i64 {
            ctx.world_mut(|w| {
                w.score += n;
                w.score
            })
        }

        fn install(api: &mut vm::api::Api<'_, '_>) {
            crate::std(api);
            api.add(add_score);
        }

        #[test]
        fn natives_reach_host_world_through_freeze_fixture() {
            let mut world = World { score: 10 };
            let mut vm =
                vm::Vm::compile("print(add_score(5)); print(add_score(3));", install).unwrap();
            let cell = vm.fixture::<WorldCell>();
            cell.freeze(&mut world, || vm.run().unwrap());
            assert_eq!(world.score, 18);
        }
    }
}
