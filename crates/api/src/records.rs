use shared::{AdtId, Literal, Ty};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ApiFunction<C> {
    pub name: String,
    pub module: Vec<String>,
    pub parameters: Vec<(String, Option<Ty>)>,
    pub return_ty: Option<Ty>,
    pub doc: String,
    #[cfg_attr(feature = "serde", serde(skip))]
    pub call: C,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ApiMethod<C> {
    pub recv_ty: Ty,
    pub name: String,
    pub parameters: Vec<(String, Option<Ty>)>,
    pub return_ty: Option<Ty>,
    pub takes_self: bool,
    pub doc: String,
    #[cfg_attr(feature = "serde", serde(skip))]
    pub call: C,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ApiConstant {
    pub name: String,
    pub module: Vec<String>,
    /// `Some` makes this an associated constant on that receiver instead of a free one.
    pub recv_ty: Option<Ty>,
    pub ty: Ty,
    pub value: Literal,
    pub doc: String,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(bound(deserialize = "C: Default")))]
pub enum ApiEntry<C> {
    Function(ApiFunction<C>),
    Method(ApiMethod<C>),
    Constant(ApiConstant),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ApiAdt {
    pub name: String,
    pub module: Vec<String>,
    pub kind: ApiAdtKind,
    pub adt_id: AdtId,
    pub doc: String,
    pub variants: Vec<ApiVariant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ApiAdtKind {
    Enum,
    Struct,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ApiVariant {
    pub name: String,
    pub layout_id: AdtId,
    pub doc: String,
    pub fields: ApiVariantFields,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ApiVariantFields {
    Unit,
    Tuple(Vec<Ty>),
    Named(Vec<(String, Ty)>),
}
