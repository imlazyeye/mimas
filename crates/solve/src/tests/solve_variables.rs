use crate::components::Ty::*;

// Inferred lets
test_ty!(
    let_infer_unit,
    "let a = ();",
    "a" => Unit
);
test_fail!(bare_null_rejected, "let a = null;");
test_ty!(
    let_infer_bools,
    "let a = true;",
    "a" => Bool, "false" => Bool
);
test_ty!(
    let_infer_int,
    "let a = 1;",
    "a" => Int
);
test_ty!(
    let_infer_float,
    "let a = 0.1;",
    "a" => Float
);
test_ty!(
    let_infer_hex,
    "let a = 0xffffff;",
    "a" => Int
);
test_ty!(
    let_infer_string,
    r#"let a = "foo";"#,
    "a" => Str
);
test_ty!(
    let_infer_empty_array,
    "let a = [];",
    "a" => array!(unknown!())
);
test_ty!(
    let_infer_array,
    "let a = [0];",
    "a" => array!(Int)
);
test_ty!(
    let_infer_empty_dictionary,
    "let a = ~{};",
    "a" => dictionary!(unknown!()),
);
test_ty!(
    let_infer_dictionary,
    "let a = ~{ a = 0 };",
    "a" => dictionary!(Int)
);
test_ty!(
    let_infer_tuple_mixed,
    "let a = (0, 1.0);",
    "a" => tuple!(Int, Float)
);

// Explicate lets
test_ty!(
    let_explicate_unit,
    "let a: () = ();",
    "a" => Unit
);
test_ty!(
    let_explicate_bools,
    "let a: bool = true;",
    "a" => Bool,
);
test_ty!(
    let_explicate_int,
    "let a: int = 1;",
    "a" => Int
);
test_ty!(
    let_explicate_float,
    "let a: float = 0.1;",
    "a" => Float
);
test_ty!(
    let_explicate_hex,
    "let a: int = 0xffffff;",
    "a" => Int
);
test_ty!(
    let_explicate_string,
    r#"let a: str = "foo";"#,
    "a" => Str
);
test_ty!(
    let_explicate_array,
    "let a: [int] = [0];",
    "a" => array!(Int)
);
test_ty!(
    let_explicate_tuple,
    "let a: (int, int, int) = (0, 1, 2);",
    "a" => tuple!(Int, Int, Int)
);
test_ty!(
    let_explicate_dictionary,
    "let a: ~{int} = ~{ a = 0 };",
    "a" => dictionary!(Int)
);
test_ty!(
    empty_array_unifies_with_typed_array,
    "let a: [int] = [];",
    "a" => array!(Int)
);
test_ty!(
    empty_dict_unifies_with_typed_dict,
    "let a: ~{int} = ~{};",
    "a" => dictionary!(Int)
);

// Shadowing
test_ty!(
    let_shadowing,
    "let a: int = 0;
     let a: bool = false;",
    "a" => Bool
);
test_ty!(
    let_shadowing_inferred,
    "let a = 0;
     let a = \"foo\";",
    "a" => Str
);
test_ty!(
    let_shadowing_after_block,
    "let a = 0;
     {
         let a = \"foo\";
     }
     let a: bool = true;",
    "a" => Bool
);

// let else
test_ty!(
    let_else,
    "let a: int? = 0;
     let b? = a else loop {};",
    "b" => Int,
);

test_ty!(
    let_else_result_unwraps,
    "let a: int! = 0;
     let b? = a else loop {};",
    "b" => Int,
);
test_ty!(
    let_else_enum_tuple_variant,
    "enum Shape { Circle(int), Square(int, int) }
     let s = Shape::Circle(3);
     let Shape::Circle(r) = s else loop {};",
    "r" => Int,
);
test_ty!(
    let_else_enum_struct_variant,
    "enum Msg { Quit, Move { x: int, y: int } }
     let m = Msg::Move { x = 1, y = 2 };
     let Msg::Move { x, y } = m else loop {};",
    "x" => Int,
);
test_success!(
    let_else_literal_guard,
    "let n = 5;
     let 5 = n else loop {};"
);
test_success!(
    let_else_or_pattern,
    "let n = 1;
     let 0 | 1 = n else loop {};"
);
test_fail!(
    let_else_needs_divergent_else,
    "let a: int? = 0; let b? = a else 0;"
);
test_fail!(
    let_else_refutable_needs_else,
    "let a: int? = 0; let b? = a;"
);
test_fail!(
    let_else_binding_absent_in_else,
    "let a: int? = 0; let b? = a else { let _c = b + 1; loop {} };"
);

// Const
test_ty!(
    const_inferred,
    "const A = 0;",
    "A" => Int,
);
test_ty!(
    const_explicate,
    "const A: int = 0;",
    "A" => Int,
);
test_fail!(const_shadow, "const A: int = 0; const A: bool = false;");
test_ty!(
    const_use_const,
    "const A: int = 1; const B: int = 1 + A;",
    "B" => Int,
);
test_ty!(
    const_array,
    "const A: [int] = [0];",
    "A" => array!(Int),
);
test_ty!(
    const_array_with_const_ident,
    "const A: int = 1;
     const B: [int] = [A, 2];",
    "B" => array!(Int),
);
test_ty!(
    const_array_nested,
    "const A: int = 1;
     const B: [int] = [A, 2];
     const C: [[int]] = [B, [A, 3]];",
    "C" => array!(array!(Int)),
);
test_fail!(const_array_with_local_ident, "let a = 1; const A = [a, 2];");
test_fail!(non_const_array, "const A = [foo()];");
test_fail!(
    const_non_literal,
    "fn foo() -> int { 0 } const BUZZ = foo();"
);
test_fail!(
    const_reassignment,
    "const A = 0;
     A = 1;"
);
test_fail!(
    const_index_assign,
    "const A = [0];
    A[0] = 1;"
);
test_fail!(const_redeclared, "const A = 0; const A = 1;");
test_fail!(const_references_local, "let a = 0; const B = a;");
test_fail!(
    const_cycle,
    "const A: int = B; const B: int = A;",
    "const A: int = A;",
);
test_fail!(use_non_module, "const A = 0; use A;");
test_fail!(assign_to_undefined, "x = 5;");
test_fail!(
    assign_to_loop_var,
    "for i in [1, 2] { i = 0; }",
    "for c in \"abc\" { c = \"x\"; }",
);

// Patterns
test_fail!(tuple_pattern_on_non_tuple, "let (a, b) = 0;");

test_ty!(
    tuple_destruction,
    "let foo = (0, 0.1, false);
    let (a, b, c) = foo;",
    "a" => Int,
    "b" => Float,
    "c" => Bool
);

test_ty!(
    nested_tuple_destruction,
    "let foo = ((0, 1), (1.1, 2.2));
    let ((a, b), (b, c)) = foo;",
    "a" => Int,
    "b" => Float,
);

// Violatons
test_fail!(violate_infered_type, "let a = 0; a = true;");
test_fail!(violate_explicate_type, "let a: int = 0; a = true;");
test_fail!(annotation_value_mismatch, "let a: int = true;");
test_fail!(typed_array_rejects_wrong_element, "let a: [int] = [1.0];");
test_fail!(read_undefined_variable, "let a = b;");
test_fail!(missing_members_in_tuple_destroy, "let (a) = (0, 1);");
test_fail!(extra_members_in_tuple_destroy, "let (a, b, c) = (0, 1);");

test_ty!(
    for_tuple_destruction,
    "let pairs = [(0, true)];
    for (a, b) in pairs {
        let _x: int = a;
        let _y: bool = b;
    }"
);

test_ty!(
    for_nested_tuple_destruction,
    "let nested = [((0, 1.0), false)];
    for ((a, b), c) in nested {
        let _x: int = a;
        let _y: float = b;
        let _z: bool = c;
    }"
);

test_fail!(
    for_missing_members_in_tuple_destroy,
    "for (a) in [(0, 1)] {}"
);
test_fail!(
    for_extra_members_in_tuple_destroy,
    "for (a, b, c) in [(0, 1)] {}"
);

// Scoping
test_ty!(
    block_hoist,
    "let a = 0;
    let b = {
        let c = a;
        c
    };",
    "b" => Int,
);
test_ty!(
    same_name_in_block,
    "let a = 0;
    {
        let a = [0];
    }",
    "a" => Int
);
test_fail!(
    read_iterator_after_loop,
    "for x in [] {}
    let a = x;"
);
test_fail!(
    read_param_after_fn,
    "fn foo(a: int) { a }
     let b = a;"
);
test_fail!(
    read_var_from_other_fn,
    "fn foo(a: int) {}
     fn bar() { a }"
);
test_fail!(
    read_var_after_if,
    "if true { let a = 0; }
     let b = a;"
);
test_fail!(
    read_while_let_binding_outside,
    "let opt: int? = 1;
     while let x = opt { break; };
     let z = x;"
);

test_fail!(
    const_min_div_rem_overflow_ices,
    "const A: int = (-9223372036854775807 - 1) % -1;",
    "const B: int = (-9223372036854775807 - 1) ~/ -1;"
);

test_fail!(
    const_struct_in_array_ices,
    "struct P { x: int } const A = [P { x = 1 }];"
);
