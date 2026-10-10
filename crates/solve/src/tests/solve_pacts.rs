use crate::components::Ty::*;

// the bare name `Foo` is deliberately not queryable as a value anymore
// (PactIsNotAValue, see pact_name_as_value_ices) -- this just smokes that the decl solves.
test_ty!(
    pact,
    "pact Foo {}",
    "0" => Int,
);

test_ty!(
    pact_method_returns_int,
    "pact Draw { fn draw(self) -> int; }
     struct Circle { r: int }
     impl Draw for Circle { fn draw(self) -> int { self.r } }
     fn render(d: Draw) -> int { d.draw() }",
    "render(Circle { r = 5 })" => Int,
);

test_ty!(
    pact_method_returns_str,
    "pact Named { fn name(self) -> str; }
     struct Dog { legs: int }
     impl Named for Dog { fn name(self) -> str { \"dog\" } }
     fn label(n: Named) -> str { n.name() }",
    "label(Dog { legs = 4 })" => Str,
);

test_ty!(
    pact_method_with_params,
    "pact Adder { fn add(self, x: int) -> int; }
     struct Base { n: int }
     impl Adder for Base { fn add(self, x: int) -> int { self.n + x } }
     fn apply(a: Adder) -> int { a.add(3) }",
    "apply(Base { n = 1 })" => Int,
);

test_ty!(
    pact_method_on_concrete_type,
    "pact Draw { fn draw(self) -> int; }
     struct Circle { r: int }
     impl Draw for Circle { fn draw(self) -> int { self.r } }
     let c = Circle { r = 9 };",
    "c.draw()" => Int,
);

test_ty!(
    pact_array_param,
    "pact Draw { fn draw(self) -> int; }
     struct Circle { r: int }
     impl Draw for Circle { fn draw(self) -> int { self.r } }
     fn first(items: [Draw]) -> int { items[0].draw() }",
    "first([Circle { r = 1 }, Circle { r = 2 }])" => Int,
);

test_ty!(
    pact_let_annotation,
    "pact Draw { fn draw(self) -> int; }
     struct Circle { r: int }
     impl Draw for Circle { fn draw(self) -> int { self.r } }
     let d: Draw = Circle { r = 4 };",
    "d.draw()" => Int,
);

test_ty!(
    pact_impl_for_enum,
    "pact Draw { fn draw(self) -> int; }
     enum Shape { Circle, Square }
     impl Draw for Shape { fn draw(self) -> int { 0 } }
     fn render(d: Draw) -> int { d.draw() }
     let s = Shape::Circle {};",
    "render(s)" => Int,
);

test_ty!(
    pact_two_impls,
    "pact Draw { fn draw(self) -> int; }
     struct Circle { r: int }
     struct Square { side: int }
     impl Draw for Circle { fn draw(self) -> int { self.r } }
     impl Draw for Square { fn draw(self) -> int { self.side } }
     let sq = Square { side = 4 };",
    "sq.side" => Int,
);

test_ty!(
    pact_self_field_access,
    "pact Area { fn area(self) -> int; }
     struct Rect { w: int, h: int }
     impl Area for Rect { fn area(self) -> int { self.w * self.h } }
     fn measure(s: Area) -> int { s.area() }",
    "measure(Rect { w = 2, h = 3 })" => Int,
);

test_ty!(
    pact_impl_after_use_site,
    "pact Draw { fn draw(self) -> int; }
     struct Circle { r: int }
     fn render(d: Draw) -> int { d.draw() }
     fn go() -> int { render(Circle { r = 1 }) }
     impl Draw for Circle { fn draw(self) -> int { self.r } }",
    "go()" => Int,
);

test_ty!(
    pact_self_coerces_to_pact,
    "pact Draw { fn draw(self) -> int; }
     struct Circle { r: int }
     fn render(d: Draw) -> int { d.draw() }
     impl Draw for Circle { fn draw(self) -> int { render(self) } }
     let c = Circle { r = 1 };",
    "c.draw()" => Int,
);

// a collection literal of mixed concrete types widens to the pacts they share, which is the
// only way to build "a list of things that implement X" without generics.
test_ty!(
    pact_array_literal_widens_to_shared_pact,
    "pact Draw { fn draw(self) -> int; }
     struct Square { s: int }
     struct Circle { r: int }
     impl Draw for Square { fn draw(self) -> int { self.s } }
     impl Draw for Circle { fn draw(self) -> int { self.r } }
     fn render_all(items: [Draw]) -> int { 0 }",
    "render_all([Square { s = 1 }, Circle { r = 2 }])" => Int,
);

// widening only fires where unification already failed, so types sharing no pact still error
test_fail!(
    mixed_array_without_shared_pact_rejected,
    "struct A { x: int }
     struct B { y: int }
     let xs = [A { x = 1 }, B { y = 2 }];"
);

test_ty!(
    pact_const_via_double_colon,
    "pact Named { const NAME: str; }
     struct Dog { legs: int }
     impl Named for Dog { const NAME = \"dog\"; }",
    "Dog::NAME" => Str,
);

test_ty!(
    pact_const_via_dot,
    "pact Identified {
         const ID: int;
     }
     struct Foo {
         tag: int,
     }
     impl Identified for Foo {
         const ID = 7;
     }
     fn get_id(v: Identified) -> int {
         v.ID
     }",
    "get_id(Foo { tag = 0 })" => Int,
);

test_ty!(
    pact_const_via_self,
    "pact Identified {
         const ID: int;
         fn next(self) -> int {
             self.ID + 1
         }
     }
     struct Foo {
         tag: int,
     }
     impl Identified for Foo {
         const ID = 7;
     }",
    "Foo { tag = 0 }.next()" => Int,
);

test_fail!(
    pact_const_via_dot_wrong_type,
    "pact Identified {
         const ID: int;
     }
     fn get_id(v: Identified) -> str {
         v.ID
     }",
);

test_fail!(
    pact_const_via_dot_in_const,
    "pact Identified {
         const ID: int;
     }
     struct Foo {
         tag: int,
     }
     impl Identified for Foo {
         const ID = 7;
     }
     fn copy(v: Identified) -> int {
         const COPY = v.ID;
         COPY
     }",
);

test_ty!(
    pact_default_method_used,
    "pact Greet { fn hello(self) -> str { \"hi\" } }
     struct Person { age: int }
     impl Greet for Person { }
     fn greeting(g: Greet) -> str { g.hello() }",
    "greeting(Person { age = 1 })" => Str,
);

test_ty!(
    pact_default_calls_sibling,
    "pact P { fn a(self) -> int; fn b(self) -> int { self.a() } }
     struct S { x: int }
     impl P for S { fn a(self) -> int { self.x } }
     fn run(p: P) -> int { p.b() }",
    "run(S { x = 5 })" => Int,
);

test_fail!(
    pact_default_body_wrong_return,
    "pact P { fn a(self) -> int; fn b(self) -> str { self.a() } }
     struct S { x: int }
     impl P for S { fn a(self) -> int { self.x } }"
);

test_ty!(
    pact_default_method_overridden,
    "pact Greet { fn hello(self) -> str { \"hi\" } }
     struct Robot { id: int }
     impl Greet for Robot { fn hello(self) -> str { \"beep\" } }
     fn greeting(g: Greet) -> str { g.hello() }",
    "greeting(Robot { id = 1 })" => Str,
);

test_ty!(
    pact_multi_bound,
    "pact Named { fn name(self) -> str; }
     pact Aged { fn age(self) -> int; }
     struct Person { years: int }
     impl Named for Person { fn name(self) -> str { \"p\" } }
     impl Aged for Person { fn age(self) -> int { self.years } }
     fn describe(v: Named + Aged) -> int { v.age() }",
    "describe(Person { years = 30 })" => Int,
);

test_fail!(
    pact_multi_bound_missing_one,
    "pact Named { fn name(self) -> str; }
     pact Aged { fn age(self) -> int; }
     struct Person { years: int }
     impl Named for Person { fn name(self) -> str { \"p\" } }
     fn describe(v: Named + Aged) -> int { 0 }
     let x = describe(Person { years = 1 });"
);

test_ty!(
    pact_bound_order_independent,
    "pact Named { fn name(self) -> str; }
     pact Aged { fn age(self) -> int; }
     struct Person { years: int }
     impl Named for Person { fn name(self) -> str { \"p\" } }
     impl Aged for Person { fn age(self) -> int { self.years } }
     fn describe(v: Aged + Named) -> int { v.age() }
     let p: Named + Aged = Person { years = 1 };",
    "describe(p)" => Int,
);

test_ty!(
    pact_bound_const_access,
    "pact Tagged {
         const TAG: int;
     }
     pact Greet {
         fn hi(self) -> str;
     }
     struct W {
         n: int,
     }
     impl Tagged for W {
         const TAG = 7;
     }
     impl Greet for W {
         fn hi(self) -> str {
             \"y\"
         }
     }
     fn tag_of(v: Tagged + Greet) -> int {
         v.TAG
     }",
    "tag_of(W { n = 1 })" => Int,
);

test_fail!(
    pact_bound_non_pact_member,
    "pact Named { fn name(self) -> str; }
     struct Thing { x: int }
     fn f(v: Named + Thing) -> int { 0 }"
);

// no-`self` pact fns are associated functions: reachable via the adt name, or on an instance
// (the receiver is just an access path, not passed as an argument).
test_ty!(
    pact_assoc_fn_via_adt,
    "pact Maker { fn make() -> int; }
     struct S { x: int }
     impl Maker for S { fn make() -> int { 5 } }",
    "S::make()" => Int,
);

test_ty!(
    pact_assoc_fn_via_instance,
    "pact Maker { fn make() -> int; }
     struct S { x: int }
     impl Maker for S { fn make() -> int { 5 } }
     let s = S { x = 1 };",
    "s.make()" => Int,
);

// a default body is compiled once and shared across impls, so `Self` is still the abstract
// pact inside it -- `Self::CONST` has no single value to resolve to. rejected, not an ICE.
test_fail!(
    pact_default_self_assoc,
    "pact Identified { const ID: int; fn print_id() -> int { Self::ID } }
     struct Foo { x: int }
     impl Identified for Foo { const ID = 0; }
     let f = Foo { x = 1 };
     f.print_id();"
);

test_fail!(
    pact_two_pacts_colliding_methods_rejected,
    "pact A { fn dup(self) -> int; }
     pact B { fn dup(self) -> str; }
     struct S { x: int }
     impl A for S { fn dup(self) -> int { self.x } }
     impl B for S { fn dup(self) -> str { \"b\" } }"
);

test_fail!(
    pact_two_pacts_colliding_defaults_rejected,
    "pact A { fn dup(self) -> int { 1 } }
     pact B { fn dup(self) -> str { \"b\" } }
     struct S { x: int }
     impl A for S { }
     impl B for S { }"
);

test_fail!(
    pact_bound_names_colliding_member,
    "pact A { fn dup(self) -> int; }
     pact B { fn dup(self) -> str; }
     fn f(v: A + B) -> int { v.dup() }"
);

test_ty!(
    pact_default_on_concrete_value,
    "pact Foo { fn bar(self) -> int { 7 } }
     struct F { x: int }
     impl Foo for F {}
     let f = F { x = 1 };",
    "f.bar()" => Int,
);

test_fail!(
    pact_impl_missing_method,
    "pact Draw { fn draw(self) -> int; }
     struct Circle { r: int }
     impl Draw for Circle { }"
);

test_fail!(
    pact_impl_wrong_return,
    "pact Draw { fn draw(self) -> int; }
     struct Circle { r: int }
     impl Draw for Circle { fn draw(self) -> str { \"x\" } }"
);

test_fail!(
    pact_impl_wrong_param,
    "pact Adder { fn add(self, x: int) -> int; }
     struct Base { n: int }
     impl Adder for Base { fn add(self, x: str) -> int { 0 } }"
);

test_fail!(
    pact_impl_const_missing,
    "pact Named { const NAME: str; }
     struct Dog { legs: int }
     impl Named for Dog { }"
);

test_fail!(
    pact_impl_unknown_pact,
    "struct Foo { tag: int }
     impl Nonexistent for Foo { }"
);

test_fail!(
    pact_nested,
    "fn main() {
         pact Marker {}
     }" => "nested `pact` declaration",
    "pact Named { fn name(self) -> str; }
     fn main() {
         pact Named { fn id(self) -> int { 0 } }
     }" => "nested `pact` declaration",
);

test_fail!(
    pact_impl_in_a_block,
    "pact Marker {}
     struct Foo { tag: int }
     fn main() { impl Marker for Foo { } }" => "nested `impl` declaration",
);

test_fail!(
    pact_unsatisfied_argument,
    "pact Draw { fn draw(self) -> int; }
     struct Circle { r: int }
     struct Square { side: int }
     impl Draw for Circle { fn draw(self) -> int { self.r } }
     fn render(d: Draw) -> int { d.draw() }
     let x = render(Square { side = 1 });"
);

test_fail!(
    pact_method_not_in_pact,
    "pact Draw { fn draw(self) -> int; }
     struct Circle { r: int }
     impl Draw for Circle { fn draw(self) -> int { self.r } }
     fn render(d: Draw) -> int { d.area() }"
);

test_fail!(
    pact_duplicate_impl,
    "pact Draw { fn draw(self) -> int; }
     struct Circle { r: int }
     impl Draw for Circle { fn draw(self) -> int { 0 } }
     impl Draw for Circle { fn draw(self) -> int { 1 } }"
);

// a pact can mix `self` methods and no-`self` associated fns; both dispatch on a concrete value.
test_ty!(
    pact_self_and_assoc_mixed,
    "pact Widget { fn area(self) -> int; fn make() -> int; }
     struct W { x: int }
     impl Widget for W { fn area(self) -> int { self.x } fn make() -> int { 0 } }
     let w = W { x = 3 };",
    "w.area()" => Int,
);

test_ty!(
    pact_self_and_assoc_mixed_static,
    "pact Widget { fn area(self) -> int; fn make() -> int; }
     struct W { x: int }
     impl Widget for W { fn area(self) -> int { self.x } fn make() -> int { 0 } }",
    "W::make()" => Int,
);

// NotAPact: `impl A for B` where A is a struct, not a pact
test_fail!(
    impl_struct_for_struct,
    "struct A {} struct B {} impl A for B {}"
);

// a bare `impl P {}` on a pact (no `for`) is rejected
test_fail!(impl_block_on_pact, "pact P {} impl P { fn f() {} }");

test_fail!(
    pact_name_as_value_ices,
    "pact Greet { fn name(self) -> str; } struct S; impl Greet for S { fn name(self) -> str { \"x\" } } fn main() { let g = Greet; }"
);

// regression check for #10
test_ty!(
    pact_assoc_fn_via_pact_value,
    "pact Maker { fn make() -> int; }
     struct S { x: int }
     impl Maker for S { fn make() -> int { 5 } }
     fn f(m: Maker) -> int { m.make() }",
    "f(S { x = 1 })" => Int,
);

test_fail!(
    pact_self_param_through_bound,
    "pact Plus {
         fn plus(self, other: Self) -> Self;
     }

     struct V { x: int }
     struct W { y: int }

     impl Plus for V {
         fn plus(self, other: V) -> V {
             V { x = self.x + other.x }
         }
     }

     impl Plus for W {
         fn plus(self, other: W) -> W {
             W { y = self.y + other.y }
         }
     }

     fn combine(a: Plus, b: Plus) -> Plus {
         a.plus(b)
     }"
);

test_fail!(
    pact_self_param_bound_arg_in_default,
    "pact Plus {
         fn plus(self, other: Self) -> Self;

         fn add_any(self, other: Plus) -> Self {
             self.plus(other)
         }
     }

     struct V { x: int }

     impl Plus for V {
         fn plus(self, other: V) -> V {
             V { x = self.x + other.x }
         }
     }"
);

test_fail!(
    pact_impl_narrows_self_to_other_implementer,
    "pact Plus {
         fn plus(self, other: Self) -> Self;
     }

     struct V { x: int }
     struct W { y: int }

     impl Plus for W {
         fn plus(self, other: W) -> W {
             W { y = self.y + other.y }
         }
     }

     impl Plus for V {
         fn plus(self, other: W) -> W {
             other
         }
     }"
);

test_fail!(
    pact_self_param_concrete_mismatch,
    "pact Plus {
         fn plus(self, other: Self) -> Self;
     }

     struct V { x: int }
     struct W { y: int }

     impl Plus for V {
         fn plus(self, other: V) -> V {
             V { x = self.x + other.x }
         }
     }

     impl Plus for W {
         fn plus(self, other: W) -> W {
             W { y = self.y + other.y }
         }
     }

     V { x = 1 }.plus(W { y = 2 });"
);

test_ty!(
    pact_mixed_self_and_plain_methods_through_bound,
    "pact Plus {
         fn plus(self, other: Self) -> Self;
         fn value(self) -> int;
     }

     struct V { x: int }

     impl Plus for V {
         fn plus(self, other: V) -> V {
             V { x = self.x + other.x }
         }

         fn value(self) -> int {
             self.x
         }
     }

     fn total(a: Plus) -> int {
         a.value()
     }",
    "total(V { x = 4 })" => Int,
);

test_ty!(
    pact_self_return_through_bound,
    "pact Dup {
         fn dup(self) -> Self;
     }

     struct V { x: int }

     impl Dup for V {
         fn dup(self) -> V {
             V { x = self.x }
         }
     }

     let d: Dup = V { x = 7 };",
    "d.dup()" => query!(Dup),
);

test_ty!(
    pact_default_passes_self_as_bound,
    "pact Plus {
         fn value(self) -> int;

         fn announced(self) -> int {
             announce(self)
         }
     }

     fn announce(p: Plus) -> int {
         p.value()
     }

     struct V { x: int }

     impl Plus for V {
         fn value(self) -> int {
             self.x
         }
     }",
    "V { x = 2 }.announced()" => Int,
);

test_ty!(
    pact_bound_param_through_bound,
    "pact Collide {
         fn hits(self, other: Collide) -> bool;
     }

     struct V { x: int }
     struct W { y: int }

     impl Collide for V {
         fn hits(self, other: Collide) -> bool {
             true
         }
     }

     impl Collide for W {
         fn hits(self, other: Collide) -> bool {
             false
         }
     }

     fn check(a: Collide, b: Collide) -> bool {
         a.hits(b)
     }",
    "check(V { x = 1 }, W { y = 2 })" => Bool,
);

// issue #59: a bound could be holding any implementer, so it never passes as one of them
test_fail!(
    pact_value_returned_as_concrete,
    "pact Draw { fn d(self) -> int { 0 } }
     struct Square { s: int }
     struct Circle { r: int }
     impl Draw for Square {}
     impl Draw for Circle {}
     fn f(d: Draw) -> Square { d }"
);

test_fail!(
    pact_value_bound_as_concrete,
    "pact Draw { fn d(self) -> int { 0 } }
     struct Square { s: int }
     impl Draw for Square {}
     let d: Draw = Square { s = 1 };
     let sq: Square = d;"
);

test_fail!(
    pact_value_passed_as_concrete,
    "pact Draw { fn d(self) -> int { 0 } }
     struct Square { s: int }
     impl Draw for Square {}
     fn side(sq: Square) -> int { sq.s }
     let d: Draw = Square { s = 1 };
     side(d);"
);

test_fail!(
    widened_array_element_as_concrete,
    "pact Draw { fn d(self) -> int { 0 } }
     struct Square { s: int }
     struct Circle { r: int }
     impl Draw for Square {}
     impl Draw for Circle {}
     let xs = [Square { s = 1 }, Circle { r = 2 }];
     let sq: Square = xs[1];"
);

test_ty!(
    pact_if_branches_widen_to_shared_pact,
    "pact Draw { fn d(self) -> int { 0 } }
     struct Square { s: int }
     struct Circle { r: int }
     impl Draw for Square {}
     impl Draw for Circle {}
     let x: Draw = if true Square { s = 1 } else Circle { r = 2 };",
    "x.d()" => Int,
);

test_ty!(
    pact_match_arms_widen_to_shared_pact,
    "pact Draw { fn d(self) -> int { 0 } }
     struct Square { s: int }
     struct Circle { r: int }
     impl Draw for Square {}
     impl Draw for Circle {}
     let x = match 1 { 0 => Square { s = 1 }, _ => Circle { r = 2 } };",
    "x.d()" => Int,
);

test_ty!(
    pact_static_ctor_branches_widen,
    "pact Draw { fn make() -> Self; fn d(self) -> int { 0 } }
     struct Square { s: int }
     struct Circle { r: int }
     impl Draw for Square { fn make() -> Self { Square { s = 1 } } }
     impl Draw for Circle { fn make() -> Self { Circle { r = 2 } } }
     let x = if true Square::make() else Circle::make();",
    "x.d()" => Int,
);

test_fail!(
    if_branches_without_shared_pact_rejected,
    "struct A { x: int }
     struct B { y: int }
     let x = if true A { x = 1 } else B { y = 2 };"
);

test_ty!(
    wider_bound_fulfills_narrower,
    "pact Draw { fn d(self) -> int { 0 } }
     pact Name { fn n(self) -> int { 1 } }
     struct Square { s: int }
     impl Draw for Square {}
     impl Name for Square {}
     fn render(d: Draw) -> int { d.d() }
     fn both(v: Draw + Name) -> int { render(v) }",
    "both(Square { s = 1 })" => Int,
);

test_fail!(
    narrower_bound_rejected_as_wider,
    "pact Draw { fn d(self) -> int { 0 } }
     pact Name { fn n(self) -> int { 1 } }
     struct Square { s: int }
     impl Draw for Square {}
     impl Name for Square {}
     fn both(v: Draw + Name) -> int { 0 }
     fn render(d: Draw) -> int { both(d) }"
);

test_ty!(
    pact_collect_widens_to_shared_pact,
    "pact Draw { fn d(self) -> int { 0 } }
     struct Square { s: int }
     struct Circle { r: int }
     impl Draw for Square {}
     impl Draw for Circle {}
     let xs = for i in 4 {
         if i == 0 { collect Square { s = i }; } else { collect Circle { r = i }; }
     };",
    "xs[0].d()" => Int,
);

test_ty!(
    each_assoc_fn,
    "pact Draw {
         fn make() -> Self;
         fn sides(n: int) -> int;
         fn reset();
     }
     struct Square;
     impl Draw for Square {
         fn make() -> Self {
             Square
         }
         fn sides(n: int) -> int {
             4 * n
         }
         fn reset() {}
     }",
    "Draw::*::make()" => array!(query!(Draw)),
    "Draw::*::sides(2)" => array!(Int),
    "Draw::*::sides(n = 2)" => array!(Int),
    "Draw::*::reset()" => array!(Unit),
);

test_ty!(
    each_constant,
    r#"pact Draw {
         const NAME: str;
     }
     struct Square;
     impl Draw for Square {
         const NAME = "square";
     }"#,
    "Draw::*::NAME" => array!(Str),
);

test_ty!(
    each_without_impls,
    "pact Draw {
         const NAME: str;
         fn make() -> Self;
     }",
    "Draw::*::make()" => array!(query!(Draw)),
    "Draw::*::NAME" => array!(Str),
);

test_multi_file!(
    each_through_a_module,
    shapes => "module @;
               pub pact Draw {
                   fn make() -> Self;
               }";
    "shapes::Draw::*::make()" => array!(TEST_SESSION.with(|s| {
        // query! can't take a path, and Draw no longer leaks out of shapes through the global pact
        // lookup. resolve shapes::Draw as a type annotation for the expected type.
        let mut s = s.borrow_mut();
        <crate::components::Ty as crate::components::TyExt>::from_annotation(parse::components::Annotation::Path(vec![parse::Ident::synthetic("shapes"), parse::Ident::synthetic("Draw")]), &mut s.0).unwrap()
    })),
);

test_fail!(
    each_method_rejected,
    "pact Draw {
         fn area(self) -> int;
     }
     Draw::*::area();"
);

test_fail!(
    each_self_param_rejected,
    "pact Draw {
         fn same(other: Self) -> bool;
     }
     struct Square;
     impl Draw for Square {
         fn same(other: Self) -> bool {
             true
         }
     }
     Draw::*::same(Square);"
);

test_fail!(
    each_fn_is_not_a_value,
    "pact Draw {
         fn make() -> Self;
     }
     let f = Draw::*::make;"
);

test_fail!(
    each_arguments_checked,
    "pact Draw {
         fn sides(n: int) -> int;
     }
     Draw::*::sides(true);",
    "pact Draw {
         fn sides(n: int) -> int;
     }
     Draw::*::sides();"
);

test_fail!(
    each_unknown_member,
    "pact Draw {}
     Draw::*::missing();"
);

test_fail!(
    each_on_a_struct,
    "struct Square;
     Square::*::make();"
);

test_fail!(
    each_on_a_pact_value,
    "pact Draw {
         fn make() -> Self;
     }
     struct Square;
     impl Draw for Square {
         fn make() -> Self {
             Square
         }
     }
     let shape: Draw = Square;
     shape::*::make();",
    "pact Draw {
         fn make() -> Self;
     }
     fn all(shape: Draw) -> [Draw] {
         shape::*::make()
     }"
);

test_fail!(
    each_result_is_not_one_impl,
    "pact Draw {
         fn make() -> Self;
     }
     struct Square;
     impl Draw for Square {
         fn make() -> Self {
             Square
         }
     }
     let squares: [Square] = Draw::*::make();"
);

test_fail!(
    pact_names_have_no_value,
    "pact P {}
     P;" => "pacts are not values",
    "pact P {}
     let p = (P);" => "pacts are not values",
    "pact P {}
     fn take(p: P) {}
     take(P);" => "pacts are not values",
);

test_multi_file_fail!(
    private_pact_is_not_global,
    a => "module a;
          pact Hidden {}",
    b => "module b;
          pub fn f(x: Hidden) {}";
);

test_multi_file!(
    pact_signatures_see_imports,
    weapons => "module @;
                use things::Thing;
                pub pact Weapon {
                    fn plain(self, thing: Thing) -> Thing;
                    fn maybe(self) -> Thing?;
                    fn tried(self) -> Thing!;
                    fn many(self) -> [Thing];
                    fn pair(self) -> (int, Thing);
                    fn with(self, make: (Thing) -> Thing) -> int;
                }",
    wand => "module @;
             use things::Thing;
             use weapons::Weapon;
             pub struct Wand {}
             impl Weapon for Wand {
                 fn plain(self, thing: Thing) -> Thing { thing }
                 fn maybe(self) -> Thing? { null }
                 fn tried(self) -> Thing! { Thing { x = 1 } }
                 fn many(self) -> [Thing] { [Thing { x = 2 }] }
                 fn pair(self) -> (int, Thing) { (0, Thing { x = 3 }) }
                 fn with(self, make: (Thing) -> Thing) -> int { make(Thing { x = 4 }).x }
             }
             pub fn held() -> Weapon { Wand {} }",
    things => "module @;
               pub struct Thing {
                   pub x: int,
               }";
    "wand::held().plain(things::Thing { x = 0 }).x" => Int,
    "wand::held().maybe()?.x" => option!(Int),
    "wand::held().tried()!.x" => Int,
    "wand::held().many()[0].x" => Int,
    "wand::held().pair().1.x" => Int,
    "wand::held().with(|thing| thing)" => Int,
);

test_multi_file!(
    pact_const_annotation_sees_imports,
    weapons => "module @;
                use things::Icon;
                pub pact Weapon {
                    const ICON: Icon;
                }
                pub fn icon(weapon: Weapon) -> Icon { weapon.ICON }",
    wand => "module @;
             use things::Icon;
             use weapons::Weapon;
             pub struct Wand {}
             impl Weapon for Wand {
                 const ICON = Icon::Star;
             }",
    things => "module @;
               pub enum Icon { Star, Moon }";
    "weapons::icon(wand::Wand {}) == things::Icon::Star" => Bool,
);

test_multi_file!(
    pact_signatures_see_braced_and_glob_imports,
    braced => "module @;
               use things::{ Thing, Icon, make };
               pub pact Braced {
                   fn icon(self, thing: Thing) -> Icon;
               }
               pub fn thing() -> Thing { make() }",
    glob => "module @;
             use things::*;
             pub pact Glob {
                 fn icon(self, thing: Thing) -> Icon;
             }",
    wand => "module @;
             use braced::Braced;
             use things::{ Thing, Icon };
             pub struct Wand {}
             impl Braced for Wand {
                 pub fn icon(self, thing: Thing) -> Icon { Icon::Star }
             }",
    orb => "module @;
            use glob::Glob;
            use things::{ Thing, Icon };
            pub struct Orb {}
            impl Glob for Orb {
                pub fn icon(self, thing: Thing) -> Icon { Icon::Moon }
            }",
    things => "module @;
               pub struct Thing {
                   pub x: int,
               }
               pub enum Icon { Star, Moon }
               pub fn make() -> Thing { Thing { x = 0 } }";
    "wand::Wand {}.icon(braced::thing()) == things::Icon::Star" => Bool,
    "orb::Orb {}.icon(braced::thing()) == things::Icon::Moon" => Bool,
);

test_multi_file!(
    pact_signatures_name_other_pacts,
    weapons => "module @;
                use ammo::Ammo;
                pub pact Weapon {
                    fn load(self, ammo: Ammo) -> Weapon;
                    fn spare(self) -> ammo::Ammo;
                    fn twin(self) -> Weapon?;
                }",
    wand => "module @;
             use ammo::Ammo;
             use weapons::Weapon;
             pub struct Wand {}
             impl Weapon for Wand {
                 pub fn load(self, ammo: Ammo) -> Weapon { self }
                 pub fn spare(self) -> Ammo { Spark {} }
                 pub fn twin(self) -> Weapon? { null }
             }
             pub struct Spark {}
             impl Ammo for Spark {
                 pub fn count(self) -> int { 1 }
             }",
    ammo => "module @;
             pub pact Ammo {
                 fn count(self) -> int;
             }";
    "wand::Wand {}.load(wand::Spark {}).spare().count()" => Int,
    "wand::Wand {}.twin()?.spare()?.count()" => option!(Int),
);

test_multi_file!(
    pact_signatures_keep_module_paths,
    weapons => "module @;
                pub pact Weapon {
                    fn plain(self, thing: things::Thing) -> things::Thing;
                }",
    wand => "module @;
             use weapons::Weapon;
             pub struct Wand {}
             impl Weapon for Wand {
                 pub fn plain(self, thing: things::Thing) -> things::Thing { thing }
             }",
    things => "module @;
               pub struct Thing {
                   pub x: int,
               }";
    "wand::Wand {}.plain(things::Thing { x = 0 }).x" => Int,
);

test_multi_file_fail!(
    pact_signature_names_private_import,
    weapons => "module @;
                use things::Thing;
                pub pact Weapon {
                    fn plain(self, thing: Thing) -> int;
                }",
    things => "module @;
               struct Thing {
                   pub x: int,
               }";
);

test_multi_file_fail!(
    pact_signature_names_missing_import,
    weapons => "module @;
                use things::Missing;
                pub pact Weapon {
                    fn plain(self, thing: Missing) -> int;
                }",
    things => "module @;
               pub struct Thing {
                   pub x: int,
               }";
);
