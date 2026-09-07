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

// pact *constants* have no dispatch -- each impl declares its own value, and a pact-typed
// receiver hasn't decided which. methods are fine (see `pact_default_method_used`); only the
// constant form is rejected, and it's rejected here rather than crashing during lowering.
test_fail!(
    pact_const_via_dot,
    "pact Identified { const ID: int; }
     struct Foo { tag: int }
     impl Identified for Foo { const ID = 7; }
     fn get_id(v: Identified) -> int { v.ID }
     get_id(Foo { tag = 0 });"
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

// same limitation as `pact_const_via_dot`, reached through a multi-pact `+` bound
test_fail!(
    pact_bound_const_access,
    "pact Tagged { const TAG: int; }
     pact Greet { fn hi(self) -> str; }
     struct W { n: int }
     impl Tagged for W { const TAG = 7; }
     impl Greet for W { fn hi(self) -> str { \"y\" } }
     fn tag_of(v: Tagged + Greet) -> int { v.TAG }
     tag_of(W { n = 1 });"
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
