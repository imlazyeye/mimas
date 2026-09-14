use std::{
    any::TypeId,
    cell::{Cell, RefCell},
};

use bevy::prelude::*;

use crate::{plugin::ScriptCtx, reflect::Catalog};
use vm::{Ctx, RtErr, Stashed, Val};

/// A component on an entity, or a resource when the entity is `None`.
type Loan = (Option<Entity>, TypeId);

/// The running hook's entity, and every copy of a component or resource it's handed out. The
/// copies get written back when the hook returns.
#[derive(Default)]
pub(crate) struct HookScope {
    entity: Cell<Option<Entity>>,
    lent: RefCell<Vec<(Loan, Stashed)>>,
}

impl HookScope {
    pub fn begin(&self, entity: Entity) {
        self.entity.set(Some(entity));
    }

    /// Ends the hook, dropping anything that wasn't written back.
    pub fn end(&self) {
        self.entity.set(None);
        self.lent.borrow_mut().clear();
    }

    pub fn entity(&self) -> Result<Entity, RtErr> {
        self.entity
            .get()
            .ok_or_else(|| RtErr::Custom("no script entity outside of a hook".into()))
    }

    /// Hands the script a copy of a component (or a resource, when `entity` is `None`). Asking
    /// again in the same hook gives back the same copy.
    pub fn lend<'gc>(
        &self,
        ctx: Ctx<'gc>,
        catalog: &Catalog,
        entity: Option<Entity>,
        id: TypeId,
    ) -> Result<Option<Val<'gc>>, RtErr> {
        // nothing would write the copy back outside of a hook
        self.entity()?;
        let loan = (entity, id);
        if let Some((_, held)) = self.lent.borrow().iter().find(|(lent, _)| *lent == loan) {
            return Ok(Some(ctx.fetch(held)));
        }
        let stored = catalog.get(id);
        let Some(value) = ctx.world(|world| stored.read(world, ctx, entity)) else {
            return Ok(None);
        };
        self.lent.borrow_mut().push((loan, ctx.stash(value)));
        Ok(Some(value))
    }

    /// Stops a copy from being written back (for `insert` and `remove`).
    pub fn forget(&self, entity: Option<Entity>, id: TypeId) {
        self.lent
            .borrow_mut()
            .retain(|(lent, _)| *lent != (entity, id));
    }

    /// Writes every copy back into the world. One that doesn't fit its type anymore just gets
    /// logged.
    pub fn write_back(&self, ctx: Ctx<'_>, catalog: &Catalog) {
        for ((entity, id), held) in self.lent.borrow_mut().drain(..) {
            let value = ctx.fetch(&held);
            if let Err(err) = ctx.world(|world| catalog.get(id).write(world, ctx, entity, value)) {
                error!("{err:?}");
            }
        }
    }
}
