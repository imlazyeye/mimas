use crate::{Ident, IntoStmt, StmtKind};

#[derive(Debug, PartialEq, Clone)]
pub struct Module {
    pub name: Ident,
}
impl Module {
    pub(crate) fn new(name: Ident) -> Self {
        Self { name }
    }
}
impl From<Module> for StmtKind {
    fn from(stmt: Module) -> Self {
        Self::Module(stmt)
    }
}

impl IntoStmt for Module {}

#[mutants::skip]
impl std::fmt::Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("module {};", self.name))
    }
}
