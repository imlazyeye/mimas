use super::ty::Ty;
use crate::{
    Solver,
    components::{DecId, TyExt},
};
use bitflags::bitflags;
use indexmap::IndexMap;
use itertools::Itertools;
use shared::Location;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

pub use shared::AdtId;

#[derive(Debug, Clone)]
pub(crate) struct Adt {
    pub name: String,
    pub variants: IndexMap<String, Variant>,
    pub impls: HashMap<String, Field>,
    pub native_overloads: HashMap<String, Vec<Field>>,
    pub flags: AdtFlags,
}
impl Adt {
    pub(crate) const STRUCT_VARIANT_NAME: &'static str = "@";

    pub fn new_struct(name: String) -> Self {
        Self {
            name,
            variants: vec![(
                Self::STRUCT_VARIANT_NAME.into(),
                Variant::Struct(StructVariant::default()),
            )]
            .into_iter()
            .collect(),
            impls: HashMap::new(),
            native_overloads: HashMap::new(),
            flags: AdtFlags::empty(),
        }
    }

    pub fn new_enum(name: String, variants: IndexMap<String, Variant>) -> Self {
        Self {
            name,
            variants,
            impls: HashMap::new(),
            native_overloads: HashMap::new(),
            flags: AdtFlags::IS_ENUM,
        }
    }

    /// A tuple struct: a single positional [`Variant::Tuple`] under the canonical struct-variant
    /// key.
    pub(crate) fn new_tuple_struct(name: String, members: Vec<Ty>) -> Self {
        Self {
            name,
            variants: std::iter::once((
                Self::STRUCT_VARIANT_NAME.into(),
                Variant::Tuple(TupleVariant {
                    members,
                    ..Default::default()
                }),
            ))
            .collect(),
            impls: HashMap::new(),
            native_overloads: HashMap::new(),
            flags: AdtFlags::empty(),
        }
    }

    pub(crate) fn new_variant_layout(name: String, variant: &Variant) -> Self {
        match variant {
            Variant::Tuple(variant) => Self::new_tuple_struct(name, variant.members.clone()),
            Variant::Struct(variant) => {
                let mut adt = Self::new_struct(name);
                adt.as_struct_mut().fields = variant.fields.clone();
                adt
            }
        }
    }

    /// True for a non-enum, non-module struct whose single variant is a
    /// positional tuple (`struct Foo(int)`).
    pub(crate) fn is_tuple_struct(&self) -> bool {
        !self.flags.contains(AdtFlags::IS_ENUM)
            && !self.flags.contains(AdtFlags::IS_MODULE)
            && matches!(self.variants.values().next(), Some(Variant::Tuple(_)))
    }

    pub(crate) fn as_singular(&self) -> Option<&Variant> {
        if !self.flags.contains(AdtFlags::IS_ENUM) {
            self.variants.values().next()
        } else {
            None
        }
    }

    pub(crate) fn as_singular_mut(&mut self) -> Option<&mut Variant> {
        if !self.flags.contains(AdtFlags::IS_ENUM) {
            self.variants.values_mut().next()
        } else {
            None
        }
    }

    /// Access the inner variant, asserting that there is only one (and therefore this is not an
    /// enum).
    pub fn as_struct(&self) -> &StructVariant {
        self.as_singular().unwrap().as_struct()
    }

    /// Fallible [Adt::as_struct]: None when this is an enum or its singular variant is a tuple.
    pub(crate) fn try_as_struct(&self) -> Option<&StructVariant> {
        match self.as_singular()? {
            Variant::Struct(s) => Some(s),
            Variant::Tuple(_) => None,
        }
    }

    pub fn as_struct_mut(&mut self) -> &mut StructVariant {
        self.as_singular_mut().unwrap().as_struct_mut()
    }

    /// Access the inner variant, asserting that there is only one (and therefore this is not an
    /// enum).
    pub(crate) fn as_tuple(&self) -> &TupleVariant {
        if !self.flags.contains(AdtFlags::IS_ENUM) {
            self.variants.values().next().unwrap().as_tuple()
        } else {
            panic!("Attempted to access an Adt with multiple variants as a singular struct!")
        }
    }

    /// Resolves this Adt's stored member types against the current substitutions. Called once at
    /// the end of the type's solve so the canonical definition carries concrete types --
    /// Ty::normalized itself stays pure and never mutates the shared definition.
    pub(crate) fn normalize_members(&mut self, solver: &Solver) {
        self.impls
            .values_mut()
            .for_each(|field| field.ty = field.ty.clone().normalized(solver));

        self.variants
            .values_mut()
            .for_each(|variant| match variant {
                Variant::Tuple(variant) => variant
                    .members
                    .iter_mut()
                    .for_each(|v| *v = v.clone().normalized(solver)),
                Variant::Struct(variant) => variant
                    .fields
                    .values_mut()
                    .for_each(|v| v.ty = v.ty.clone().normalized(solver)),
            });
    }
}

#[mutants::skip]
impl std::fmt::Display for Adt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name.clone();
        if !self.flags.contains(AdtFlags::IS_ENUM) {
            let fields = &self.variants.values().next().unwrap();
            f.pad(&format!("{name} {{ {fields} }}"))
        } else {
            f.pad(&format!(
                "{name} {{ {} }}",
                self.variants
                    .iter()
                    .map(|(key, value)| format!("{key}{value}"))
                    .join(", ")
            ))
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Field {
    pub ty: Ty,
    pub constant: bool,
    pub declaration_location: Location,
    pub dec: DecId,
}

impl Field {
    pub(crate) fn new(ty: Ty, declaration_location: Location, dec: DecId) -> Self {
        Field {
            ty,
            constant: false,
            declaration_location,
            dec,
        }
    }

    pub(crate) fn new_constant(ty: Ty, declaration_location: Location, dec: DecId) -> Self {
        Field {
            ty,
            constant: true,
            declaration_location,
            dec,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Variant {
    Tuple(TupleVariant),
    Struct(StructVariant),
}

impl Variant {
    pub(crate) fn layout(&self) -> Option<AdtId> {
        match self {
            Variant::Tuple(variant) => variant.layout,
            Variant::Struct(variant) => variant.layout,
        }
    }

    pub(crate) fn set_layout(&mut self, layout: AdtId) {
        match self {
            Variant::Tuple(variant) => variant.layout = Some(layout),
            Variant::Struct(variant) => variant.layout = Some(layout),
        }
    }

    pub(crate) fn dec(&self) -> Option<DecId> {
        match self {
            Variant::Tuple(variant) => variant.dec,
            Variant::Struct(variant) => variant.dec,
        }
    }

    /// Source span of this variant's name, set only for source-defined enums (native variants
    /// leave it `None`).
    pub(crate) fn location(&self) -> Option<Location> {
        match self {
            Variant::Tuple(variant) => variant.location,
            Variant::Struct(variant) => variant.location,
        }
    }

    pub(crate) fn set_dec(&mut self, dec: DecId) {
        match self {
            Variant::Tuple(variant) => variant.dec = Some(dec),
            Variant::Struct(variant) => variant.dec = Some(dec),
        }
    }

    pub(crate) fn as_struct(&self) -> &StructVariant {
        match self {
            Variant::Tuple(_) => {
                panic!("Attempted to access an Adt variant as a struct when it is a tuple!")
            }
            Variant::Struct(s) => s,
        }
    }

    pub(crate) fn as_struct_mut(&mut self) -> &mut StructVariant {
        match self {
            Variant::Tuple(_) => {
                panic!("Attempted to access an Adt variant as a struct when it is a tuple!")
            }
            Variant::Struct(s) => s,
        }
    }

    pub(crate) fn as_tuple(&self) -> &TupleVariant {
        match self {
            Variant::Tuple(t) => t,
            Variant::Struct(_) => {
                panic!("Attempted to access an Adt variant as a tuple when it is a struct!")
            }
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        match self {
            Variant::Tuple(t) => t.members.is_empty(),
            Variant::Struct(s) => s.fields.is_empty(),
        }
    }
}

#[mutants::skip]
impl std::fmt::Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Variant::Tuple(members) => f.pad(&format!(
                "({})",
                members.members.iter().map(|v| v.to_string()).join(", ")
            )),
            Variant::Struct(map) => f.pad(
                &map.fields
                    .iter()
                    .map(|(key, value)| format!("{{ {key}: {} }}", value.ty))
                    .join(", "),
            ),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TupleVariant {
    pub members: Vec<Ty>,
    pub layout: Option<AdtId>,
    pub dec: Option<DecId>,
    pub location: Option<Location>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StructVariant {
    pub fields: Fields,
    pub layout: Option<AdtId>,
    pub dec: Option<DecId>,
    pub location: Option<Location>,
}

impl StructVariant {
    pub(crate) fn insert(&mut self, key: String, adt_field: Field) {
        self.fields.insert(key, adt_field);
    }

    pub(crate) fn ty(&self, key: &str) -> Option<Ty> {
        self.fields.get(key).map(|field| field.ty.clone())
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Fields(pub IndexMap<String, Field>);

#[mutants::skip]
impl std::fmt::Display for Fields {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            f.pad("{}")
        } else {
            f.pad(&format!(
                "{{ {} }}",
                self.iter()
                    .map(|(name, field)| format!("{name}: {}", field.ty))
                    .join(", ")
            ))
        }
    }
}

impl Deref for Fields {
    type Target = IndexMap<String, Field>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Fields {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

bitflags! {
     pub(crate) struct AdtFlags: u32 {
        const IS_ENUM = 1 << 0;
        const IS_MODULE = 1 << 1;
        const IS_BUILTIN = 1 << 2;
    }
}
