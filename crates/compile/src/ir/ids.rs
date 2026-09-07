use crate::ir::{Ir, IrDisplay};

shared::id!(pub Local, pub InstId, pub BlockId);
pub use shared::BodyId;

#[mutants::skip]
impl std::fmt::Display for Local {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "l{}", self.index())
    }
}

#[mutants::skip]
impl std::fmt::Display for InstId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "i{}", self.index())
    }
}

#[mutants::skip]
impl IrDisplay for BlockId {
    #[allow(unused)]
    fn ir_display(&self, ir: &Ir) -> String {
        cfg_select! {
            feature = "logging" => ir
                .current_body()
                .block_names
                .get(self)
                .cloned()
                .unwrap_or_default(),
            _ => format!("b{}", self.0),
        }
    }
}
