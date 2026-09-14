//! Runs mimas scripts in a Bevy app. See the [Bevy](https://mim.as/extension/bevy.html) guide.
mod api {
    mod ecs;
    mod input;
    mod install;

    pub(crate) use ecs::install_message;
    pub(crate) use install::install;
}
mod compile;
mod hooks;
mod plugin;
mod reflect {
    mod builder;
    mod catalog;
    mod convert;
    mod kind;
    mod stored;

    pub use builder::BEVY_CRATES;
    pub(crate) use catalog::Catalog;
    pub(crate) use convert::Convert;
}
mod script;
mod types;
mod writeback;

pub use plugin::{MimasApp, MimasPlugin, MimasSystems, ScriptCtx};
pub use script::{MimasScript, Script, ScriptError};
pub use types::Entity;

#[doc(hidden)]
pub use reflect::BEVY_CRATES;

/// Everything an app needs to set up scripts.
pub mod prelude {
    pub use super::{
        MimasApp, MimasPlugin, MimasScript, MimasSystems, Script, ScriptCtx, ScriptError,
    };
}
