mod ir {
    #[allow(clippy::module_inception)]
    mod ir;
    pub use ir::*;
    mod emit;
    use emit::*;
    mod place;
    use place::*;
    mod body;
    mod constant;
    mod ids;
    mod inst;
    mod ops;
    pub use body::*;
    pub use constant::*;
    pub use ids::*;
    pub use inst::*;
    pub use ops::*;

    // rexport this
    pub use parse::AccessKind;
}
pub use ir::*;

#[macro_use]
mod codegen {
    mod clean;
    mod compiler;
    #[macro_use]
    mod op;
    mod program;
    mod render;
    pub use compiler::*;
    pub use op::*;
    pub use program::*;
}
pub use codegen::*;

#[cfg(test)]
mod tests {
    #[macro_use]
    mod compile_test_utils;

    mod bytecode;
    mod ir_tests;
}
