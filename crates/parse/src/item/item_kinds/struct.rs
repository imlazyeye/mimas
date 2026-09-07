use crate::{
    components::Annotation,
    expr::Ident,
    item::{IntoItem, ItemKind},
};

use itertools::Itertools;
use shared::Location;

/// Representation of a struct declaration in mimas. Visibility lives on the wrapping [Item];
/// field-level visibility is on `StructField`.
#[derive(Debug, PartialEq, Clone)]
pub struct Struct {
    pub name: Ident,
    pub fields: Vec<StructField>,
}
impl Struct {
    #[cfg(test)]
    pub(crate) fn new(name: Ident, fields: Vec<StructField>) -> Self {
        Self { name, fields }
    }
}

impl From<Struct> for ItemKind {
    fn from(s: Struct) -> Self {
        Self::Struct(s)
    }
}
impl IntoItem for Struct {}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: FieldKey,
    pub location: Location,
    pub annotation: Annotation,
    pub public: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldKey {
    Ident(Ident),
    Int(usize),
}

#[mutants::skip]
impl std::fmt::Display for FieldKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldKey::Ident(ident) => f.pad(&ident.to_string()),
            FieldKey::Int(i) => f.pad(&i.to_string()),
        }
    }
}

#[mutants::skip]
impl std::fmt::Display for Struct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!(
            "struct {} {{ {} }}",
            self.name,
            self.fields
                .iter()
                .map(
                    |StructField {
                         name, annotation, ..
                     }| format!("{name}: {annotation}")
                )
                .join(", ")
        ))
    }
}

impl PartialEq for StructField {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.annotation == other.annotation
    }
}
