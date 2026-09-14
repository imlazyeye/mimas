use crate::{MimasStruct, vm::RtErr};

/// A Bevy entity as scripts see it, carried as its bits. Bevy's own `Entity` can't implement
/// mimas's conversion trait from outside mimas, so this stands in for it.
#[derive(MimasStruct, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Entity(pub i64);

impl From<bevy::prelude::Entity> for Entity {
    fn from(entity: bevy::prelude::Entity) -> Self {
        Self(entity.to_bits() as i64)
    }
}

impl TryFrom<Entity> for bevy::prelude::Entity {
    type Error = RtErr;

    fn try_from(entity: Entity) -> Result<Self, RtErr> {
        bevy::prelude::Entity::try_from_bits(entity.0 as u64)
            .ok_or_else(|| RtErr::InvalidArgument(format!("Entity({}) isn't an entity", entity.0)))
    }
}
