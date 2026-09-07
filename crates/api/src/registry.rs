use shared::{AdtId, Id};
use std::{any::TypeId, collections::HashMap};

#[derive(Debug, Clone)]
pub struct AdtBinding {
    pub adt_id: AdtId,
    pub variant_layout_ids: Vec<AdtId>,
}

pub struct Registry {
    next_id: u32,
    bindings: HashMap<TypeId, AdtBinding>,
}

// corresponds to the core types we put methods/asc fns on -- add a new one, this blows up!
pub const BUILTIN_ADT_COUNT: u32 = 6;

impl Registry {
    pub fn new() -> Self {
        Self {
            next_id: BUILTIN_ADT_COUNT,
            bindings: HashMap::new(),
        }
    }

    pub fn alloc(&mut self) -> AdtId {
        let id = AdtId::new(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn bind<T: 'static>(&mut self, binding: AdtBinding) {
        self.bindings.insert(TypeId::of::<T>(), binding);
    }

    pub fn get<T: 'static>(&self) -> Option<&AdtBinding> {
        self.bindings.get(&TypeId::of::<T>())
    }

    pub fn bindings(&self) -> &HashMap<TypeId, AdtBinding> {
        &self.bindings
    }

    pub fn into_bindings(self) -> HashMap<TypeId, AdtBinding> {
        self.bindings
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
