use crate::{
    StructField,
    components::Annotation,
    expr::Ident,
    item::{IntoItem, ItemKind},
};
use itertools::Itertools;

/// Representation of an enum declaration in mimas. Visibility lives on the wrapping [Item].
#[derive(Debug, PartialEq, Clone)]
pub struct Enum {
    pub head: Ident,
    /// The members of this enum.
    pub members: Vec<(Ident, Member)>,
}
impl Enum {
    #[cfg(test)]
    pub(crate) fn new(name: Ident, members: Vec<(Ident, Member)>) -> Self {
        Self {
            head: name,
            members,
        }
    }
}

impl From<Enum> for ItemKind {
    fn from(enu: Enum) -> Self {
        Self::Enum(enu)
    }
}
impl IntoItem for Enum {}

#[mutants::skip]
impl std::fmt::Display for Enum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!(
            "enum {} {{ {} }}",
            self.head,
            self.members
                .iter()
                .map(|(ident, member)| match member {
                    Member::Tuple(members) => format!(
                        "{ident}({})",
                        members.iter().map(|v| v.to_string()).join(", ")
                    ),
                    Member::Struct(fields) => {
                        format!(
                            "{ident} {{ {} }}",
                            fields
                                .iter()
                                .map(
                                    |StructField {
                                         name, annotation, ..
                                     }| {
                                        format!("{ident} {name}: {annotation}")
                                    },
                                )
                                .join(", ")
                        )
                    }
                })
                .join(", ")
        ))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Member {
    Tuple(Vec<Annotation>),
    Struct(Vec<StructField>),
}
