use std::{collections::HashMap, ops::Range};

use shared::IdVec;

use crate::{ApiAdt, ApiConstant, ApiEntry, ApiFunction, ApiMethod, Intrinsic, NativeId, Registry};

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(bound(deserialize = "C: Default")))]
pub struct Library<C> {
    natives: IdVec<NativeId, ApiEntry<C>>,
    #[cfg_attr(feature = "serde", serde(skip))]
    intrinsics: HashMap<NativeId, Intrinsic>,
    adts: Vec<ApiAdt>,
    #[cfg_attr(feature = "serde", serde(skip))]
    registry: Registry,
    /// The adt ids and natives (by index) the standard library took. We use this in other places
    /// to carve out the library to exclusively user types.
    std: Option<(Range<u32>, Range<usize>)>,
}

impl<C> Library<C> {
    pub fn new() -> Self {
        Self {
            natives: IdVec::new(),
            intrinsics: HashMap::new(),
            adts: Vec::new(),
            registry: Registry::new(),
            std: None,
        }
    }

    pub fn natives(&self) -> impl Iterator<Item = (NativeId, &ApiEntry<C>)> {
        self.natives.iter()
    }

    pub fn adts(&self) -> &[ApiAdt] {
        &self.adts
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut Registry {
        &mut self.registry
    }

    pub fn into_registry(self) -> Registry {
        self.registry
    }

    pub fn push_adt(&mut self, adt: ApiAdt) {
        self.adts.push(adt);
    }

    pub fn function(&mut self, f: ApiFunction<C>) -> NativeId {
        self.natives.push(ApiEntry::Function(f))
    }

    pub fn method(&mut self, m: ApiMethod<C>) -> NativeId {
        self.natives.push(ApiEntry::Method(m))
    }

    pub fn constant(&mut self, c: ApiConstant) -> NativeId {
        self.natives.push(ApiEntry::Constant(c))
    }

    pub fn mark_instrinsic(&mut self, nid: NativeId, i: Intrinsic) {
        self.intrinsics.insert(nid, i);
    }

    pub fn intrinsics(&self) -> &HashMap<NativeId, Intrinsic> {
        &self.intrinsics
    }

    /// Marks `adts` (by id) and `natives` (by index) as the standard library.
    pub fn mark_std(&mut self, adts: Range<u32>, natives: Range<usize>) {
        self.std = Some((adts, natives));
    }

    /// The library without what [`Library::mark_std`] marked. The ids stay as they were, so it's
    /// for reading what the host added, not for checking scripts against.
    pub fn without_std(mut self) -> Self {
        let Some((adts, natives)) = self.std.take() else {
            return self;
        };
        let mut kept = IdVec::new();
        for (index, entry) in self.natives.into_values().enumerate() {
            if !natives.contains(&index) {
                kept.push(entry);
            }
        }
        self.natives = kept;
        self.adts
            .retain(|adt| !adts.contains(&(adt.adt_id.index() as u32)));
        self
    }
}

impl<C> Default for Library<C> {
    fn default() -> Self {
        Self::new()
    }
}
