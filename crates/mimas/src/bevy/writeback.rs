use std::{
    any::TypeId,
    cell::{Cell, RefCell},
};

use bevy::prelude::*;

use crate::{
    bevy::{plugin::ScriptCtx, reflect::Catalog},
    vm::{Ctx, RtErr, Val},
};

/// A value on loan to the script, and where it goes back to when the hook returns.
struct WriteBack<'gc> {
    value: Val<'gc>,
    entity: Option<Entity>,
    type_id: TypeId,
}

/// The hook in progress: its entity, and what it has lent the script so far. Scripts work on
/// copies of components and resources, so every copy handed out is remembered here and written
/// back into the world when the hook returns. A per-Vm fixture.
#[derive(Default)]
pub(crate) struct HookScope {
    entity: Cell<Option<Entity>>,
    backs: RefCell<Vec<WriteBack<'static>>>,
}

impl HookScope {
    /// Starts a hook for `entity`.
    pub fn begin(&self, entity: Entity) {
        self.entity.set(Some(entity));
    }

    /// Ends the hook. Anything still on loan is dropped without being written back.
    pub fn end(&self) {
        self.entity.set(None);
        self.backs.borrow_mut().clear();
    }

    /// The entity whose hook is running.
    pub fn entity(&self) -> Option<Entity> {
        self.entity.get()
    }

    /// Lends the script a copy of a component or resource. Asking again in the same hook returns
    /// the same copy, so two copies can't race each other on the way back.
    pub fn lend<'gc>(
        &self,
        ctx: Ctx<'gc>,
        entity: Option<Entity>,
        type_id: TypeId,
        fetch: impl FnOnce(&mut World) -> Option<Val<'gc>>,
    ) -> Result<Option<Val<'gc>>, RtErr> {
        // the copy is written back when the hook returns, so there has to be a hook
        ctx.entity()?;
        let lent = self
            .backs
            .borrow()
            .iter()
            .find(|back| back.entity == entity && back.type_id == type_id)
            .map(|back| back.value);
        if let Some(value) = lent {
            // SAFETY: see below
            return Ok(Some(unsafe {
                std::mem::transmute::<Val<'static>, Val<'gc>>(value)
            }));
        }
        let Some(value) = ctx.world(fetch)? else {
            return Ok(None);
        };
        let back = WriteBack {
            value,
            entity,
            type_id,
        };
        // SAFETY: an entry exists only between `begin` and `end` and is read back only within the
        // same hook, which is one arena session, so its value is never collected in between. `end`
        // drops what a faulted hook left behind without reading it.
        let back = unsafe { std::mem::transmute::<WriteBack<'_>, WriteBack<'static>>(back) };
        self.backs.borrow_mut().push(back);
        Ok(Some(value))
    }

    /// Drops the pending write-back for this value, so an explicit `insert` or `remove` in the
    /// same hook stands.
    pub fn forget(&self, entity: Option<Entity>, type_id: TypeId) {
        self.backs
            .borrow_mut()
            .retain(|back| !(back.entity == entity && back.type_id == type_id));
    }

    /// Puts everything lent out during the hook back into the world. A value that no longer fits
    /// its type is logged and skipped rather than failing the hook.
    pub fn write_back<'gc>(&self, ctx: Ctx<'gc>, catalog: &Catalog) {
        let backs = std::mem::take(&mut *self.backs.borrow_mut());
        // SAFETY: see `lend`
        let backs =
            unsafe { std::mem::transmute::<Vec<WriteBack<'static>>, Vec<WriteBack<'gc>>>(backs) };
        for back in backs {
            let wrote = ctx
                .world(|world| catalog.write(world, ctx, back.type_id, back.entity, back.value))
                .and_then(|result| result);
            if let Err(err) = wrote {
                error!("{err:?}");
            }
        }
    }
}
