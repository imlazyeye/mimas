use crate::Ty;

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

impl Literal {
    pub fn ty(&self) -> Ty {
        match self {
            Literal::Null => Ty::Null,
            Literal::Bool(_) => Ty::Bool,
            Literal::Int(_) => Ty::Int,
            Literal::Float(_) => Ty::Float,
            Literal::Str(_) => Ty::Str,
        }
    }
}
