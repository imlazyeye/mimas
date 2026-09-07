use mimas::{mimas, vm::conversion::Raisable};

#[mimas]
fn greet(name: String) -> String {
    format!("hello, {name}!")
}

#[mimas]
struct Player {
    name: String,
    health: i64,
}

#[mimas]
impl Player {
    fn damage(&mut self, amount: i64) {
        self.health -= amount;
    }
}

#[mimas]
enum Element {
    Fire,
    Water,
    Earth,
}

#[mimas(game)]
const MAX_HEALTH: i64 = 100;

#[mimas(game)]
fn weakness(element: Element) -> Element {
    match element {
        Element::Fire => Element::Water,
        Element::Water => Element::Earth,
        Element::Earth => Element::Fire,
    }
}

#[mimas]
fn foo() -> Raisable<()> {
    Raisable::Raised("uh oh!".into())
}

fn main() {
    let mut vm = mimas::compile_source(include_str!("../scripts/extension.mim")).unwrap();
    vm.run().unwrap();
}
