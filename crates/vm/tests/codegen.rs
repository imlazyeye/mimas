#[macro_use]
mod vm_test_utils;

use vm::Captured::*;

test_vm!(
    opt_nested_else_if_chain,
    "fn classify(n: int) -> int {
        if n < 0 { 0 }
        else if n == 0 { 1 }
        else if n < 10 { 2 }
        else if n < 100 { 3 }
        else { 4 }
    }",
    "classify(-5)" => Int(0),
    "classify(0)" => Int(1),
    "classify(5)" => Int(2),
    "classify(50)" => Int(3),
    "classify(500)" => Int(4),
);

test_vm!(
    opt_conditional_phi_forwarder,
    "fn pick(c: bool, x: int) -> int {
        let y = if c { x + 100 } else { x };
        y * 2
    }",
    "pick(true, 5)" => Int(210),
    "pick(false, 5)" => Int(10),
);

test_vm!(
    opt_match_expression_phi,
    "enum Dir { N, S, E, W }
     fn delta(d: Dir) -> int {
        match d {
            Dir::N => 1,
            Dir::S => 0 - 1,
            Dir::E => 10,
            Dir::W => 0 - 10,
        }
    }",
    "delta(Dir::N)" => Int(1),
    "delta(Dir::S)" => Int(-1),
    "delta(Dir::E)" => Int(10),
    "delta(Dir::W)" => Int(-10),
);

// an infinite loop with a return inside; the trailing expression is unreachable and must be
// dropped by dce without disturbing the live path
test_vm!(
    opt_loop_return_dead_tail,
    "fn loopy(n: int) -> int {
        let acc = 0;
        loop {
            acc += n;
            if acc > 100 { return acc; }
        }
        acc + 9999
    }",
    "loopy(7)" => Int(105),
    "loopy(30)" => Int(120),
);

// live phi at the top, then a large unreachable region (nested ifs, a match, two loops) that
// dce must clear entirely
test_vm!(
    opt_dce_clears_large_dead_region,
    "fn live(n: int) -> int {
        let base = if n > 0 { n * 2 } else { 0 - n };
        return base + 1;

        let junk = 0;
        for i in 1000 {
            if i % 2 == 0 {
                junk += i;
            } else if i % 3 == 0 {
                junk -= i;
            } else {
                junk = match i % 5 {
                    0 => junk + 1,
                    1 => junk - 1,
                    _ => junk,
                };
            }
        }
        while junk > 0 {
            junk -= 1;
        }
        junk + 123456
    }",
    "live(10)" => Int(21),
    "live(-4)" => Int(5),
);

test_vm!(
    opt_absolve_phi_with_dead_code,
    "fn handler(c: bool) -> int! {
        if c { 7 } else { raise \"no\" }
     }
     fn use_it(c: bool) -> int {
        let v = handler(c) absolve |_| 0 - 1;
        return v * 10;

        let junk = 0;
        for i in 50 {
            if i % 2 == 0 { junk += i; } else { junk -= i; }
        }
        while junk > 0 { junk -= 1; }
        junk + 7777
    }",
    "use_it(true)" => Int(70),
    "use_it(false)" => Int(-10),
);

// keep1/keep2/keep3 stay live across two calls; if a callee frame overlaps the caller's window
// (base computed too low) those writes get stomped and the sum is wrong
test_vm!(
    hw_caller_regs_survive_call,
    "fn helper(n: int) -> int { let a = n; let b = n; let c = n; let d = n; a + b + c + d }
     fn outer(seed: int) -> int {
        let keep1 = seed + 1;
        let keep2 = seed + 2;
        let r = helper(seed);
        let keep3 = seed + 3;
        let r2 = helper(seed + 1);
        keep1 + keep2 + keep3 + r + r2
     }",
    "outer(10)" => Int(120),
);

test_vm!(
    hw_deep_recursion,
    "fn sumto(n: int) -> int { if n == 0 { 0 } else { n + sumto(n - 1) } }",
    "sumto(500)" => Int(125250),
);

test_vm!(
    hw_mutual_recursion_varied_frames,
    "fn evens(n: int) -> int { if n == 0 { 0 } else { n + odds(n - 1) } }
     fn odds(n: int) -> int { let a = n - 1; let b = a + 1; if n == 0 { 0 } else { b + evens(n - 1) } }",
    "evens(50)" => Int(1275),
);

test_vm!(
    hw_repeated_call_window_reuse,
    "fn square(n: int) -> int { let s = n * n; s }
     let total = 0;
     for i in 100 { total += square(i); }",
    "total" => Int(328350),
);

test_vm!(
    hw_alternating_pollute_clean,
    "fn poison(n: int) -> int { let a = n + 700; let b = n + 800; let c = n + 900; a + b + c }
     fn clean(n: int) -> int { let x = n; let y = n; x + y }
     let total = 0;
     for i in 50 { poison(i); total += clean(i); }",
    "total" => Int(2450),
);

test_vm!(
    hw_nested_call_args,
    "fn add3(a: int, b: int, c: int) -> int { a + b + c }
     fn dbl(n: int) -> int { n * 2 }",
    "add3(dbl(dbl(1)), add3(1, 2, dbl(3)), dbl(5))" => Int(23),
);

// deep recursion that allocates a heap node at every level, built and summed 60 times -- forces gc
// while reused and freshly-grown slots hold live Gc pointers. a grown tail left uninitialized (or a
// reused slot dropped from the trace) corrupts the heap. uses only builtins, since the test vm has
// no stdlib natives.
test_vm!(
    hw_gc_pressure_recursive_alloc,
    "struct Node { value: int, next: Node? }
     fn build(n: int) -> Node? { if n == 0 { null } else { Node { value = n, next = build(n - 1) } } }
     fn sum(node: Node?) -> int { if let nd? = node { nd.value + sum(nd.next) } else { 0 } }
     let total = 0;
     for k in 60 { let lst = build(k); total += sum(lst); }",
    "total" => Int(35990),
);
