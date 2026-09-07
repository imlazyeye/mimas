use itertools::Itertools;

use crate::{
    Expr,
    components::{Annotation, Binding},
    expr::Ident,
    item::{IntoItem, ItemKind},
};

/// Representation of function declaration in mimas. Visibility lives on the wrapping [Item].
#[derive(Debug, PartialEq, Clone)]
pub struct Function {
    /// The name, if any, of this function. Anonymous functions do not have names.
    pub name: Ident,
    /// The parameters of this function.
    pub parameters: Vec<Binding>,
    /// The body of the function declaration.
    pub body: Expr,
    /// The type bound for the return.
    pub return_type: Option<Annotation>,
}
impl Function {
    /// Creates a new function declaration.
    #[cfg(test)]
    pub(crate) fn new(
        name: Ident,
        // todo, dont think this should be binding, or at least, this shouldnt contain a pat
        parameters: Vec<Binding>,
        return_type: Option<Annotation>,
        body: Expr,
    ) -> Self {
        Self {
            name,
            parameters,
            body,
            return_type,
        }
    }

    pub fn is_method(&self) -> bool {
        self.parameters
            .first()
            .and_then(|b| b.left.as_ident())
            .is_some_and(|i| i.lexeme == "self")
    }
}
impl From<Function> for ItemKind {
    fn from(function: Function) -> Self {
        Self::Function(function)
    }
}
impl IntoItem for Function {}

#[mutants::skip]
impl std::fmt::Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let param_str = self.parameters.iter().join(", ");
        if let Some(ret) = &self.return_type {
            f.pad(&format!(
                "fn {}({param_str}) -> {} {}",
                self.name, ret, self.body
            ))
        } else {
            f.pad(&format!("fn {}({param_str}) {}", self.name, self.body))
        }
    }
}

/// One of the inbuilt functions mimas's runtime supports.
#[derive(Debug, PartialEq, Clone)]
pub enum FfiFn {
    Print,
    Stringify,
}

impl TryFrom<String> for FfiFn {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "print" => Ok(FfiFn::Print),
            "stringify" => Ok(FfiFn::Stringify),
            _ => Err(()),
        }
    }
}
