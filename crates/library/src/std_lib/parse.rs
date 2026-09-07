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
    api.add_method(Value::as_dict);
    api.add_method(Value::as_array);
    api.add_method(Value::as_str);
    api.add_method(Value::as_float);
}

#[native]
fn from_json(v: &str) -> Raisable<Value> {
    serde_json::from_str::<serde_json::Value>(v)
        .map(Into::into)
        .into()
}

#[native]
fn to_json(v: Value) -> Raisable<String> {
    serde_json::to_string(&serde_json::Value::from(v)).into()
}

#[derive(MimasEnum)]
enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Value {
    #[native]
    fn as_dict(value: Value) -> Option<HashMap<String, Value>> {
        let Self::Object(dict) = value else {
            return None;
        };
        Some(dict)
    }

    #[native]
    fn as_array(value: Value) -> Option<Vec<Value>> {
        let Self::Array(arr) = value else { return None };
        Some(arr)
    }

    #[native]
    fn as_str(value: Value) -> Option<String> {
        let Self::String(s) = value else { return None };
        Some(s)
    }

    #[native]
    fn as_float(value: Value) -> Option<f64> {
        let Self::Number(f) = value else { return None };
        Some(f)
    }
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
