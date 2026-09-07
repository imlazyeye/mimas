use std::collections::HashMap;

use crate::Id;

crate::id!(pub StrId);

#[derive(Debug, Default, Clone)]
pub struct StrInterner {
    strings: Vec<Box<str>>,
    ids: HashMap<Box<str>, StrId>,
}

impl StrInterner {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            ids: HashMap::new(),
        }
    }

    pub fn intern(&mut self, value: &str) -> StrId {
        if let Some(&id) = self.ids.get(value) {
            return id;
        }

        let id = StrId::new(self.strings.len() as u32);
        let value: Box<str> = value.into();

        self.strings.push(value.clone());
        self.ids.insert(value, id);

        id
    }

    pub fn get(&self, id: StrId) -> &str {
        &self.strings[id.index()]
    }

    pub fn id(&self, value: &str) -> Option<StrId> {
        self.ids.get(value).copied()
    }

    pub fn contains(&self, value: &str) -> bool {
        self.ids.contains_key(value)
    }
}
