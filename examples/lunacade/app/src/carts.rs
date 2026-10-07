use serde_json::{Value, json};

include!(concat!(env!("OUT_DIR"), "/carts.rs"));

/// The carts that ship with lunacade as the page takes them, an array of `{id, files}` in the order
/// of their folders under `examples/lunacade/carts`.
pub(crate) fn examples() -> String {
    let carts: Vec<Value> = CARTS
        .iter()
        .map(|(id, files)| {
            let files: Value = files
                .iter()
                .map(|(path, text)| (path.to_string(), Value::from(*text)))
                .collect::<serde_json::Map<_, _>>()
                .into();
            json!({ "id": id, "files": files })
        })
        .collect();
    Value::Array(carts).to_string()
}
