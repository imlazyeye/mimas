// Full-IR snapshot tests were retired: they pinned the exact shape of an intermediate
// representation, so every *intentional* lowering/codegen change broke a pile of them at once --
// high churn, low signal. Lowering correctness is covered end-to-end by the vm tests (they run the
// program and check the result). What lives here instead:
//   - lowering *shape* properties that a result-only test can't see (e.g. "match uses a jump
//     table"),
//   - optimizer guarantees that a result-only test *structurally* can't see, because the
//     optimization produces the same result with fewer/cheaper ops (DVE, fall-through elision,
//     constant hoisting). A vm test passes whether or not these fired; these don't.

use super::compile_test_utils::{compile_ops, lower_ir};
use crate::Op;

// match: jump-table vs chain. lots of specific rules govern when a match can use a jump table, and
// getting them wrong is destructive, so pin down what does and doesn't switch.

#[test]
fn match_lowers_to_switch() {
    let ir = lower_ir(
        "enum Color { Red, Green, Blue }
         fn f(c: Color) -> int { match c { Color::Red => 0, Color::Green => 1, Color::Blue => 2 } }",
    );
    assert!(ir.contains("switch"), "expected a jump table:\n{ir}");
    assert!(!ir.contains("is_instance"), "should not chain:\n{ir}");
}

#[test]
fn ints_can_also_switch() {
    let ir = lower_ir(
        "let a = 0;
        match a { 0 => {}, 1 => {}, 5 => {}, _ => {} }",
    );
    assert!(ir.contains("switch"), "expected a jump table:\n{ir}");
    assert!(!ir.contains("is_instance"), "should not chain:\n{ir}");
}

#[test]
fn match_or_pattern_lowers_to_single_switch() {
    let ir = lower_ir(
        "enum Color { Red, Green, Blue }
         fn f(c: Color) -> int { match c { Color::Red | Color::Green => 0, Color::Blue => 1 } }",
    );
    assert_eq!(
        ir.matches("switch").count(),
        1,
        "expected exactly one switch:\n{ir}"
    );
    assert!(!ir.contains("is_instance"), "should not chain:\n{ir}");
}

#[test]
fn guarded_match_falls_back_to_chain() {
    let ir = lower_ir(
        "enum Color { Red, Green }
         fn f(c: Color, b: bool) -> int { match c { Color::Red if b => 0, _ => 1 } }",
    );
    assert!(!ir.contains("switch"), "a guard can't switch:\n{ir}");
    assert!(
        ir.contains("is_instance"),
        "expected an is_instance chain:\n{ir}"
    );
}

// optimizer guarantees (op-stream, post-optimization)

#[test]
fn dead_if_statement_leaves_no_jump() {
    // `if` as a statement with an empty body yields no value and has no effect: dead-value elim
    // drops the merge value, and fall-through elision drops the jump into the merge.
    let ops = compile_ops("let a = 0;\nif a == 0 {}");
    assert!(
        !ops.iter().any(|op| matches!(op, Op::Jump { .. })),
        "no leftover unconditional jump expected:\n{ops:#?}"
    );
}

#[test]
fn empty_branches_thread_away() {
    // both arms are empty forwarder blocks; threading + fall-through should leave no stray jumps.
    let ops = compile_ops("let a = 1;\nif a == 1 {} else {}");
    assert!(
        !ops.iter().any(|op| matches!(op, Op::Jump { .. })),
        "empty forwarder blocks should be threaded out:\n{ops:#?}"
    );
}

#[test]
fn overflow_const_does_not_fold() {
    // not folded: the add survives (as an immediate, `MAX` in a reg) to fault at runtime.
    let ops = compile_ops("let a = 9223372036854775807 + 1;");
    assert!(
        ops.iter()
            .any(|op| matches!(op, Op::AddIntImm { val: 1, .. })),
        "an overflowing const add must be left for the runtime fault:\n{ops:#?}"
    );
}

#[test]
fn while_body_falls_through() {
    let ops = compile_ops("let s = 0;\nlet i = 0;\nwhile i < 10 { s = s + i; i = i + 1; }");
    let jumps = ops
        .iter()
        .filter(|op| matches!(op, Op::Jump { .. }))
        .count();
    assert_eq!(jumps, 1, "only the back-edge jump should remain:\n{ops:#?}");
}

#[test]
fn for_body_falls_through() {
    let ops = compile_ops("let s = 0;\nfor i in 10 { s = s + i; }");
    let jumps = ops
        .iter()
        .filter(|op| matches!(op, Op::Jump { .. }))
        .count();
    let for_nexts = ops
        .iter()
        .filter(|op| matches!(op, Op::ForNext { .. }))
        .count();
    // the loop tail (incr + test + back-edge) fuses into one ForNext, so no plain Jump survives.
    assert_eq!(jumps, 0, "the back-edge is now a ForNext:\n{ops:#?}");
    assert_eq!(
        for_nexts, 1,
        "the loop tail should be a single ForNext:\n{ops:#?}"
    );
}

#[test]
fn loop_invariant_arith_const_becomes_immediate() {
    // `7` folds into the add as an immediate -- inlined, never loaded into a register at all.
    let ops = compile_ops("let s = 0;\nfor i in 100 { s = s + 7; }");
    assert!(
        ops.iter()
            .any(|op| matches!(op, Op::AddIntImm { val: 7, .. })),
        "loop-invariant arith constant should inline as an immediate:\n{ops:#?}"
    );
    assert!(
        !ops.iter().any(|op| matches!(
            op,
            Op::LoadConst {
                constant: crate::Constant::Int(7),
                ..
            }
        )),
        "the immediate constant should not be loaded into a register:\n{ops:#?}"
    );
}

// const-fold correctness: each arithmetic/comparison/bit operator must fold to the right
// constant. these pin `BinOp::eval_int` / `eval_float` -- a result-only vm test can't, since
// the runtime has its own (separately-tested) arithmetic and would compute the right answer
// even if folding were wrong.
fn folds_to(src: &str, expected: crate::Constant) {
    let ops = compile_ops(&format!("let x = {src};"));
    assert!(
        ops.iter()
            .any(|op| matches!(op, Op::LoadConst { constant, .. } if *constant == expected)),
        "`{src}` should fold to {expected:?}:\n{ops:#?}"
    );
}

#[test]
fn const_fold_int_ops() {
    use crate::Constant::{Bool, Int};
    folds_to("7 + 3", Int(10));
    folds_to("7 - 3", Int(4));
    folds_to("7 * 3", Int(21));
    folds_to("7 ~/ 3", Int(2));
    folds_to("7 % 3", Int(1));
    folds_to("6 & 3", Int(2));
    folds_to("6 | 1", Int(7));
    folds_to("6 ^ 3", Int(5));
    folds_to("1 << 4", Int(16));
    folds_to("32 >> 2", Int(8));
    folds_to("3 < 5", Bool(true));
    folds_to("5 < 3", Bool(false));
    folds_to("3 <= 3", Bool(true));
    folds_to("5 > 3", Bool(true));
    folds_to("3 >= 5", Bool(false));
    folds_to("3 == 3", Bool(true));
    folds_to("3 != 3", Bool(false));
}

#[test]
fn const_fold_float_ops() {
    use crate::Constant::{Bool, Float};
    folds_to("1.5 + 2.5", Float(4.0));
    folds_to("5.0 - 2.0", Float(3.0));
    folds_to("2.0 * 3.0", Float(6.0));
    folds_to("7.0 / 2.0", Float(3.5));
    folds_to("1.5 < 2.5", Bool(true));
    folds_to("2.5 <= 2.5", Bool(true));
    folds_to("2.5 > 1.5", Bool(true));
    folds_to("1.5 >= 2.5", Bool(false));
    folds_to("1.5 == 1.5", Bool(true));
    folds_to("1.5 != 2.5", Bool(true));
}
