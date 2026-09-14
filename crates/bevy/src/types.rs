use macros::MimasStruct;
use vm::RtErr;

/// Bevy's `Entity` for scripts, stored as its bits. The orphan rule stops us from implementing
/// `MimasType` for Bevy's own `Entity`.
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
