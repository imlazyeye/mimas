use shared::{AdtId, Id, Ty};
use std::{any::TypeId, collections::HashMap};

#[derive(Debug, Clone)]
pub struct AdtBinding {
    pub adt_id: AdtId,
    pub variant_layout_ids: Vec<AdtId>,
}

#[derive(Clone)]
pub struct Registry {
    pub next_id: u32,
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
        self.bind_id(TypeId::of::<T>(), binding);
    }

    pub fn bind_id(&mut self, type_id: TypeId, binding: AdtBinding) {
        self.bindings.insert(type_id, binding);
    }

    pub fn get<T: 'static>(&self) -> Option<&AdtBinding> {
        self.get_id(TypeId::of::<T>())
    }

    pub fn get_id(&self, type_id: TypeId) -> Option<&AdtBinding> {
        self.bindings.get(&type_id)
    }

    pub fn ty_of_id(&self, type_id: TypeId) -> Option<Ty> {
        self.get_id(type_id).map(|binding| Ty::Adt(binding.adt_id))
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
