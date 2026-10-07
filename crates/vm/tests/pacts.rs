// pact (mimas's trait analogue) execution coverage; lives in the vm crate so `cargo miri test`
// exercises the pact codegen plus the unsafe decode/register paths. the #[ignore]d cases at the
// bottom are known-broken shapes that encode the correct expected result -- drop the ignore once
// the lowering is fixed and they become regression guards.

#[macro_use]
mod vm_test_utils;

use vm::Captured::*;

test_vm!(
    pact_method_dispatch,
    "pact Greet { fn name(self) -> str; }
     struct Dog { tag: str }
     impl Greet for Dog { fn name(self) -> str { self.tag } }
     let d = Dog { tag = \"rex\" };",
    "d.name()" => str!("rex")
);

test_vm!(
    pact_assoc_const,
    "pact Named { const KIND: str; }
     struct Dog { tag: str }
     impl Named for Dog { const KIND: str = \"canine\"; }",
    "Dog::KIND" => str!("canine")
);

test_vm!(
    pact_const_dispatch,
    r#"pact Named {
         const KIND: str;
         const LEGS: int;
         fn shout(self) -> str {
             self.KIND + "!"
         }
     }
     struct Dog;
     struct Bird;
     enum Fish {
         Trout,
         Eel(int),
     }
     impl Named for Dog {
         const KIND = "canine";
         const LEGS = 4;
     }
     impl Named for Bird {
         const KIND = "avian";
         const LEGS = 2;
     }
     impl Named for Fish {
         const KIND = "fish";
         const LEGS = 0;
     }
     fn kind(thing: Named) -> str {
         thing.KIND
     }
     fn legs(things: [Named]) -> int {
         let sum = 0;
         for thing in things {
             sum += thing.LEGS;
         }
         sum
     }"#,
    "kind(Dog)" => str!("canine"),
    "kind(Bird)" => str!("avian"),
    "kind(Fish::Eel(3))" => str!("fish"),
    "legs([Dog, Bird, Fish::Trout, Dog])" => Int(10),
    "Bird.shout()" => str!("avian!"),
);

test_vm!(
    pact_const_dispatch_through_bound,
    "pact Tagged {
         const TAG: int;
     }
     pact Aged {
         const AGE: int;
     }
     struct Old;
     struct Young;
     impl Tagged for Old {
         const TAG = 1;
     }
     impl Aged for Old {
         const AGE = 90;
     }
     impl Tagged for Young {
         const TAG = 2;
     }
     fn sum(thing: Tagged + Aged) -> int {
         thing.TAG + thing.AGE
     }",
    "sum(Old)" => Int(91),
);

test_vm!(
    pact_const_dispatch_on_option,
    r#"pact Named {
         const KIND: str;
     }
     struct Dog;
     impl Named for Dog {
         const KIND = "canine";
     }
     fn kind(thing: Named?) -> str {
         thing?.KIND ?? "nothing"
     }"#,
    "kind(Dog)" => str!("canine"),
    "kind(null)" => str!("nothing"),
);

test_vm!(
    pact_method_dispatch_on_option,
    r#"pact Named {
         fn shout(self) -> str;
     }
     struct Dog;
     impl Named for Dog {
         fn shout(self) -> str {
             "woof!"
         }
     }
     fn shout(thing: Named?) -> str {
         thing?.shout() ?? "silence"
     }"#,
    "shout(Dog)" => str!("woof!"),
    "shout(null)" => str!("silence"),
);

test_vm!(
    pact_int_method_in_arithmetic,
    "pact Score { fn score(self) -> int; }
     struct P { v: int }
     impl Score for P { fn score(self) -> int { self.v * 2 } }
     let p = P { v = 5 };",
    "p.score() + 1" => Int(11)
);

test_vm!(
    pact_dispatch_by_type,
    "pact Greet { fn g(self) -> str; }
     struct A; struct B;
     impl Greet for A { fn g(self) -> str { \"a\" } }
     impl Greet for B { fn g(self) -> str { \"b\" } }
     let a = A; let b = B;",
    "a.g()" => str!("a"),
    "b.g()" => str!("b")
);

test_vm!(
    pact_multiple_required_methods,
    "pact PT { fn one(self) -> int; fn two(self) -> int; }
     struct S { a: int }
     impl PT for S { fn one(self) -> int { self.a } fn two(self) -> int { self.a + 1 } }
     let s = S { a = 10 };",
    "s.one() + s.two()" => Int(21)
);

test_vm!(
    pact_method_calls_self_method,
    "pact PT { fn base(self) -> int; fn doubled(self) -> int; }
     struct S { a: int }
     impl PT for S { fn base(self) -> int { self.a } fn doubled(self) -> int { self.base() * 2 } }
     let s = S { a = 7 };",
    "s.doubled()" => Int(14)
);

test_vm!(
    pact_default_method_overridden,
    "pact D { fn val(self) -> int { 0 } }
     struct S { a: int }
     impl D for S { fn val(self) -> int { self.a } }
     let s = S { a = 9 };",
    "s.val()" => Int(9)
);

test_vm!(
    pact_enum_impl,
    "pact Tag { fn tag(self) -> str; }
     enum E { X, Y }
     impl Tag for E { fn tag(self) -> str { match self { E::X => \"x\", E::Y => \"y\" } } }
     let e = E::Y;",
    "e.tag()" => str!("y")
);

test_vm!(
    pact_inherited_default_method,
    "pact Greet { fn name(self) -> str; fn hello(self) -> str { \"hi\" } }
     struct Dog { tag: str }
     impl Greet for Dog { fn name(self) -> str { self.tag } }
     let d = Dog { tag = \"rex\" };",
    "d.hello()" => str!("hi")
);

// pact-typed parameter: single implementor, so dispatch takes the monomorphic fast path.
test_vm!(
    pact_typed_parameter,
    "pact Greet { fn name(self) -> str; }
     struct Dog { tag: str }
     impl Greet for Dog { fn name(self) -> str { self.tag } }
     fn announce(g: Greet) -> str { g.name() }",
    "announce(Dog { tag = \"rex\" })" => str!("rex")
);

// struct field bound by multiple pacts (`A + B`): only S satisfies both, so monomorphic.
test_vm!(
    pact_multi_bound_field,
    "pact A { fn a(self) -> int; } pact B { fn b(self) -> int; }
     struct S { x: int }
     impl A for S { fn a(self) -> int { self.x } }
     impl B for S { fn b(self) -> int { self.x } }
     struct W { item: A + B }
     let w = W { item = S { x = 1 } };",
    "w.item.a()" => Int(1)
);

test_vm!(
    pact_typed_dispatch_chain,
    "pact Greet { fn g(self) -> str; }
     struct A; struct B;
     impl Greet for A { fn g(self) -> str { \"a\" } }
     impl Greet for B { fn g(self) -> str { \"b\" } }
     fn call(x: Greet) -> str { x.g() }",
    "call(A)" => str!("a"),
    "call(B)" => str!("b")
);

test_vm!(
    pact_typed_dispatch_enum,
    "pact Tag { fn t(self) -> int; }
     enum E { X, Y }
     struct S;
     impl Tag for E { fn t(self) -> int { match self { E::X => 1, E::Y => 2 } } }
     impl Tag for S { fn t(self) -> int { 0 } }
     fn tag(x: Tag) -> int { x.t() }",
    "tag(E::Y)" => Int(2),
    "tag(E::X)" => Int(1),
    "tag(S)" => Int(0)
);

// one shared default chunk, two different receivers
test_vm!(
    pact_default_shared_across_impls,
    "pact Greet { fn name(self) -> str; fn hello(self) -> str { \"hi\" } }
     struct Dog { tag: str }
     struct Cat { tag: str }
     impl Greet for Dog { fn name(self) -> str { self.tag } }
     impl Greet for Cat { fn name(self) -> str { self.tag } }
     let d = Dog { tag = \"rex\" };
     let c = Cat { tag = \"tom\" };",
    "d.hello() + c.hello()" => str!("hihi")
);

test_vm!(
    pact_default_overridden,
    "pact Greet { fn name(self) -> str; fn hello(self) -> str { \"hi\" } }
     struct Dog { tag: str }
     impl Greet for Dog {
         fn name(self) -> str { self.tag }
         fn hello(self) -> str { \"woof\" }
     }
     let d = Dog { tag = \"rex\" };",
    "d.hello()" => str!("woof")
);

test_vm!(
    pact_default_with_params,
    "pact Greet { fn name(self) -> str; fn repeat(self, n: int) -> int { n * 2 } }
     struct Dog { tag: str }
     impl Greet for Dog { fn name(self) -> str { self.tag } }
     let d = Dog { tag = \"rex\" };",
    "d.repeat(21)" => Int(42)
);

// end-to-end version of `pact_array_literal_widens_to_shared_pact`: a mixed-type array really
// does dispatch per element at runtime, not just type-check.
test_vm!(
    pact_array_dispatches_per_element,
    "pact Draw { fn draw(self) -> str; }
     struct Square { s: int }
     struct Circle { r: int }
     impl Draw for Square { fn draw(self) -> str { \"sq\" } }
     impl Draw for Circle { fn draw(self) -> str { \"ci\" } }
     fn render_all(items: [Draw]) -> str {
         let out = \"\";
         for item in items { out += item.draw(); }
         out
     }",
    "render_all([Square { s = 1 }, Circle { r = 2 }])" => str!("sqci")
);

// regression check for #10
test_vm!(
    pact_assoc_fn_via_value,
    "pact Maker { fn make(n: int) -> int; }
    struct A;
    impl Maker for A { fn make(n: int) -> int { n + 1 } }
    fn run(m: Maker) -> int { m.make(10) }
    let a = A;",
    "a.make(1)" => Int(2),
    "run(a)" => Int(11),
);

test_vm!(
    pact_self_param_concrete_call,
    "pact Plus {
         fn plus(self, other: Self) -> Self;
     }

     struct V { x: int }

     impl Plus for V {
         fn plus(self, other: V) -> V {
             V { x = self.x + other.x }
         }
     }",
    "V { x = 1 }.plus(V { x = 2 }).x" => Int(3)
);

test_vm!(
    pact_default_passes_self_as_self,
    "pact Plus {
         fn plus(self, other: Self) -> Self;
         fn value(self) -> int;

         fn double(self) -> Self {
             self.plus(self)
         }
     }

     struct V { x: int }
     struct W { y: int }

     impl Plus for V {
         fn plus(self, other: V) -> V {
             V { x = self.x + other.x }
         }

         fn value(self) -> int {
             self.x
         }
     }

     impl Plus for W {
         fn plus(self, other: W) -> W {
             W { y = self.y + other.y }
         }

         fn value(self) -> int {
             self.y
         }
     }

     fn dbl(p: Plus) -> int {
         p.double().value()
     }",
    "V { x = 3 }.double().x" => Int(6),
    "dbl(V { x = 3 })" => Int(6),
    "dbl(W { y = 4 })" => Int(8)
);

test_vm!(
    each_assoc_fn,
    "pact Shape {
         fn sides() -> int;
     }
     struct Square;
     struct Triangle;
     impl Shape for Square {
         fn sides() -> int {
             4
         }
     }
     impl Shape for Triangle {
         fn sides() -> int {
             3
         }
     }
     fn total() -> int {
         let sum = 0;
         for sides in Shape::*::sides() {
             sum += sides;
         }
         sum
     }",
    "4 in Shape::*::sides()" => Bool(true),
    "3 in Shape::*::sides()" => Bool(true),
    "total()" => Int(7),
);

test_vm!(
    each_result_dispatches,
    "pact Shape {
         fn make() -> Self;
         fn sides(self) -> int;
     }
     struct Square;
     struct Triangle;
     impl Shape for Square {
         fn make() -> Self {
             Square
         }
         fn sides(self) -> int {
             4
         }
     }
     impl Shape for Triangle {
         fn make() -> Self {
             Triangle
         }
         fn sides(self) -> int {
             3
         }
     }
     fn total() -> int {
         let sum = 0;
         for shape in Shape::*::make() {
             sum += shape.sides();
         }
         sum
     }",
    "total()" => Int(7),
);

test_vm!(
    each_shares_its_arguments,
    "pact Shape {
         fn mark(log: [int], n: int);
     }
     struct Square;
     struct Triangle;
     impl Shape for Square {
         fn mark(log: [int], n: int) {
             log[0] += n;
         }
     }
     impl Shape for Triangle {
         fn mark(log: [int], n: int) {
             log[0] += n;
         }
     }
     fn next(counter: [int]) -> int {
         counter[0] += 1;
         counter[0]
     }
     let counter = [0];
     let log = [0];
     let marked = Shape::*::mark(log, next(counter));",
    "marked" => array!(Null, Null),
    "counter" => array!(Int(1)),
    "log" => array!(Int(2)),
);

test_vm!(
    each_default_assoc_fn,
    "pact Shape {
         fn sides() -> int {
             1
         }
     }
     struct Blob;
     struct Square;
     impl Shape for Blob {}
     impl Shape for Square {
         fn sides() -> int {
             4
         }
     }
     fn total() -> int {
         let sum = 0;
         for sides in Shape::*::sides() {
             sum += sides;
         }
         sum
     }",
    "total()" => Int(5),
);

test_vm!(
    each_counts_an_enum_once,
    "pact Shape {
         fn sides() -> int;
     }
     enum Poly {
         Five,
         Six,
     }
     impl Shape for Poly {
         fn sides() -> int {
             5
         }
     }",
    "Shape::*::sides()" => array!(Int(5)),
);

test_vm!(
    each_constant,
    r#"struct Point {
         x: int,
     }
     pact Shape {
         const NAME: str;
         const ORIGIN: Point;
     }
     struct Square;
     struct Triangle;
     impl Shape for Square {
         const NAME = "square";
         const ORIGIN = Point { x = 4 };
     }
     impl Shape for Triangle {
         const NAME = "triangle";
         const ORIGIN = Point { x = 3 };
     }
     fn total() -> int {
         let sum = 0;
         for origin in Shape::*::ORIGIN {
             sum += origin.x;
         }
         sum
     }"#,
    r#""square" in Shape::*::NAME"# => Bool(true),
    r#""triangle" in Shape::*::NAME"# => Bool(true),
    "total()" => Int(7),
);

test_vm!(
    each_without_impls,
    "pact Shape {
         const NAME: str;
         fn make() -> Self;
     }",
    "Shape::*::make()" => array!(),
    "Shape::*::NAME" => array!(),
);

test_vm!(
    each_across_files,
    files {
        shapes => "module @;
                   pub pact Shape {
                       fn sides() -> int;
                   }
                   struct Square;
                   impl Shape for Square {
                       fn sides() -> int {
                           4
                       }
                   }
                   pub fn total() -> int {
                       let sum = 0;
                       for sides in Shape::*::sides() {
                           sum += sides;
                       }
                       sum
                   }",
        main => "use shapes::Shape;
                 struct Triangle;
                 impl Shape for Triangle {
                     fn sides() -> int {
                         3
                     }
                 }
                 let TEST_VALUE = shapes::total();",
    } => Int(7),
);
