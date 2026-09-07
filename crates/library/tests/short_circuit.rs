#[macro_use]
mod test_runner;

test_run!(
    and_or_short_circuit,
    "let log: [int] = [];
     let effect = |v: bool| { log.push(1); v };
     let a = false && effect(true);
     let b = true && effect(true);
     let c = true || effect(true);
     let d = false || effect(true);",
    "log.len()" => "2",
);

test_run!(
    coalesce_short_circuits,
    "let log: [int] = [];
     let effect = || { log.push(1); 9 };
     let present: int? = 5;
     let absent: int? = null;
     let a = present ?? effect();
     let b = absent ?? effect();",
    "log.len()" => "1",
);

test_run!(
    option_call_skips_args_on_null,
    "struct S { x: int }
     impl S { fn m(self, v: int) -> int { v } }
     let log: [int] = [];
     let effect = || { log.push(1); 3 };
     let some: S? = S { x = 1 };
     let none: S? = null;
     let a = some?.m(effect());
     let b = none?.m(effect());",
    "log.len()" => "1",
);
