use host::Library;
use mimas::vm::{Vm, api::Api};

use crate::api::{
    gfx::{self, Color},
    input::{self, Button, Mouse},
    luna,
};

/// The API a cart is compiled against, without a Vm to run it. Building one makes a throwaway
/// Vm, so a caller that checks carts again and again should keep it around.
pub fn library() -> Library<()> {
    Vm::new().install_library(install)
}

/// Installs the sandboxed std and the cart API. Everything is registered explicitly and never
/// through `#[mimas]`, so it doesn't depend on how the host links.
pub(crate) fn install<'gc>(api: &mut Api<'_, 'gc>) {
    mimas::library::sandboxed(api);
    // a type has to be registered before anything that names it
    api.add_adt::<Color>();
    api.add_adt::<Button>();
    api.add_adt::<Mouse>();
    luna::install(api);
    gfx::install(api);
    input::install(api);
}
