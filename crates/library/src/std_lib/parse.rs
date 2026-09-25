use std::collections::HashMap;

use macros::native;
use vm::{MimasEnum, api::Api, conversion::Raisable};

pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    {
        let mut m = api.module("std::parse");
        m.add_adt::<Value>();
        m.add(from_json);
        m.add(to_json);
    }
    api.add_method(as_dict);
    api.add_method(as_array);
    api.add_method(as_str);
    api.add_method(as_float);
}

/// Parses `text` as JSON and returns it as a `Value`. Raises if `text` isn't valid JSON, with a
/// message giving the line and column of the problem.
///
/// ```mimas
/// use std::parse;
///
/// let data = parse::from_json("{\"name\": \"ada\", \"hp\": 30}")!;
/// let name = data.as_dict()!["name"]!.as_str()!; // "ada"
/// ```
#[native]
fn from_json(text: &str) -> Raisable<Value> {
    serde_json::from_str::<serde_json::Value>(text)
        .map(Into::into)
        .into()
}

/// Returns `value` written as compact JSON, with no spaces or line breaks. Object keys are written
/// in alphabetical order, and a whole number has no decimal point (`3.0` is written as `3`). A
/// `NaN` or infinite number, which JSON can't represent, is written as `null`.
///
/// ```mimas
/// use std::parse;
/// use std::parse::Value;
///
/// let player = Value::Object(~{
///     name = Value::String("ada"),
///     level = Value::Number(3.0),
/// });
/// let text = parse::to_json(player)!; // "{\"level\":3,\"name\":\"ada\"}"
/// ```
#[native]
fn to_json(value: Value) -> Raisable<String> {
    serde_json::to_string(&serde_json::Value::from(value)).into()
}

/// A JSON value, as `from_json` returns it and `to_json` takes it. Match on it to handle each kind
/// of value, or use the `as_` methods to get the kind you expect.
///
/// ```mimas
/// use std::parse;
/// use std::parse::Value;
///
/// let data = parse::from_json("{\"alive\": true}")!;
/// let alive = match data.as_dict()!["alive"]! {
///     Value::Bool(b) => b,
///     _ => false,
/// };
/// ```
#[derive(MimasEnum)]
pub enum Value {
    /// JSON's `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number. JSON doesn't tell integers and decimals apart, and both are stored as a `float`.
    Number(f64),
    /// A string.
    String(String),
    /// An array of values.
    Array(Vec<Value>),
    /// An object, keyed by string. The keys of an object from `from_json` are in no particular
    /// order.
    Object(HashMap<String, Value>),
}

/// Returns the entries of an `Object`, or `null` for any other kind of value. The keys are in no
/// particular order.
///
/// ```mimas
/// use std::parse;
///
/// let data = parse::from_json("{\"hp\": 30}")!;
/// let hp = data.as_dict()!["hp"]!.as_float()!; // 30.0
/// ```
#[native]
fn as_dict(value: Value) -> Option<HashMap<String, Value>> {
    let Value::Object(dict) = value else {
        return None;
    };
    Some(dict)
}

/// Returns the elements of an `Array`, or `null` for any other kind of value.
///
/// ```mimas
/// use std::parse;
///
/// let list = parse::from_json("[1, 2, 3]")!;
/// let n = list.as_array()!.len(); // 3
/// ```
#[native]
fn as_array(value: Value) -> Option<Vec<Value>> {
    let Value::Array(arr) = value else {
        return None;
    };
    Some(arr)
}

/// Returns the text of a `String`, or `null` for any other kind of value.
///
/// ```mimas
/// use std::parse;
///
/// let a = parse::from_json("\"hi\"")!.as_str(); // "hi"
/// let b = parse::from_json("12")!.as_str();     // null
/// ```
#[native]
fn as_str(value: Value) -> Option<String> {
    let Value::String(s) = value else { return None };
    Some(s)
}

/// Returns the number in a `Number`, or `null` for any other kind of value. Every JSON number
/// comes back as a `float`. Call `to_int` on the result when you need an `int`.
///
/// ```mimas
/// use std::parse;
///
/// let a = parse::from_json("12")!.as_float();           // 12.0
/// let b = parse::from_json("12")!.as_float()!.to_int(); // 12
/// ```
#[native]
fn as_float(value: Value) -> Option<f64> {
    let Value::Number(f) = value else { return None };
    Some(f)
}

impl From<serde_json::Value> for Value {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(b) => Self::Bool(b),
            // saftey: without arbitrary_precision enabled, this is always Some
            serde_json::Value::Number(number) => Self::Number(number.as_f64().unwrap()),
            serde_json::Value::String(s) => Self::String(s),
            serde_json::Value::Array(values) => {
                Self::Array(values.into_iter().map(Into::into).collect())
            }

            serde_json::Value::Object(map) => {
                Self::Object(map.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
        }
    }
}

impl From<Value> for serde_json::Value {
    fn from(value: Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Bool(b) => Self::Bool(b),
            // mimas has one number type (f64); emit whole values as ints so consumers
            // that expect integers (e.g. mdBook section numbers) round-trip.
            Value::Number(f) if f.is_finite() && f.fract() == 0.0 && f.abs() < 9.007e15 => {
                Self::Number((f as i64).into())
            }
            Value::Number(f) => serde_json::Number::from_f64(f).map_or(Self::Null, Self::Number),
            Value::String(s) => Self::String(s),
            Value::Array(a) => Self::Array(a.into_iter().map(Into::into).collect()),
            Value::Object(o) => Self::Object(o.into_iter().map(|(k, v)| (k, v.into())).collect()),
        }
    }
}
