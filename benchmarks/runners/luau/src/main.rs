use std::fs;

use mlua::Lua;

fn main() {
    let path = std::env::args().nth(1).expect("usage: luau-run <script.lua>");
    let source = fs::read_to_string(&path).expect("failed to read script");

    let lua = Lua::new();
    lua.load(&source)
        .set_name(&path)
        .exec()
        .expect("runtime error");
}
