use crate::{Ctx, Val};

pub trait VmDisplay<'gc> {
    fn vm_display(&self, ctx: Ctx<'gc>) -> String;
}

impl<'gc> VmDisplay<'gc> for Val<'gc> {
    fn vm_display(&self, ctx: Ctx<'gc>) -> String {
        ctx.display(*self)
    }
}
