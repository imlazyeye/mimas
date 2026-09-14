use shared::{Literal, Ty};
use vm::api::Api;

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    let mut m = api.module("std::math");
    m.constant(
        "PI",
        Ty::Float,
        Literal::Float(std::f64::consts::PI),
        "the ratio of a circle's circumference to its diameter",
    );
    m.constant(
        "TAU",
        Ty::Float,
        Literal::Float(std::f64::consts::TAU),
        "a full turn in radians, twice PI",
    );
    m.constant(
        "E",
        Ty::Float,
        Literal::Float(std::f64::consts::E),
        "Euler's number",
    );
}
