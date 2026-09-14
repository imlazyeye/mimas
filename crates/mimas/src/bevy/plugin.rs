use std::any::TypeId;

use bevy::{
    asset::LoadedFolder,
    ecs::message::Message,
    prelude::*,
    reflect::{FromReflect, GetTypeRegistration, Typed},
};

use crate::{
    bevy::{api, compile, hooks, script, writeback::HookScope},
    vm::{
        Ctx, RtErr,
        api::NativeFnReg,
        freeze::{Freeze, FreezeCell, FreezeError},
    },
};

/// Runs `.mim` assets as scripts. Every file under [`scripts`](Self::scripts) loads at startup.
/// Files that declare a module compile into every script, and each other file compiles into a Vm
/// of its own, run for the entities whose [`Script`](super::Script) points at it.
pub struct MimasPlugin {
    /// The folder under `assets/` to load at startup, `"scripts"` by default. With `None`, scripts
    /// still compile as their assets arrive.
    pub scripts: Option<String>,
}

impl Default for MimasPlugin {
    fn default() -> Self {
        Self {
            scripts: Some("scripts".into()),
        }
    }
}

/// The systems that compile and run scripts. Order your own systems against this set.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MimasSystems;

impl Plugin for MimasPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<script::MimasScript>()
            .init_asset_loader::<script::MimasScriptLoader>()
            .add_message::<script::ScriptError>()
            .init_resource::<ScriptRegistry>()
            .add_systems(
                Update,
                (compile::compile_scripts, hooks::run(hooks::Hook::Update))
                    .chain()
                    .in_set(MimasSystems),
            )
            .add_systems(
                FixedUpdate,
                hooks::run(hooks::Hook::FixedUpdate).in_set(MimasSystems),
            );
        // the loader has to be registered before the folder looks for files it can load
        let folder = self.scripts.as_ref().map(|path| {
            app.world()
                .resource::<AssetServer>()
                .load_folder(path.clone())
        });
        app.world_mut().resource_mut::<ScriptRegistry>().folder = folder;
        app.world_mut().insert_non_send(hooks::ScriptVms::default());
    }
}

/// What the app added for scripts beyond the types reflection finds on its own.
#[derive(Resource, Default)]
pub(crate) struct ScriptRegistry {
    pub installers: Vec<NativeFnReg>,
    pub messages: Vec<TypeId>,
    pub folder: Option<Handle<LoadedFolder>>,
}

/// Adds what scripts can reach beyond the reflected components and resources, which they find on
/// their own.
pub trait MimasApp {
    /// Runs `install` on every script's `Api` after the built-in natives. Natives it registers
    /// reach Bevy through [`ScriptCtx`].
    fn script_installer(&mut self, install: NativeFnReg) -> &mut Self;

    /// Lets scripts read and write a message as `T::read()` and `T::write(message)`. Messages need
    /// this call because Bevy can only read them through their concrete type.
    fn script_message<T>(&mut self) -> &mut Self
    where
        T: Message + FromReflect + Typed + GetTypeRegistration;
}

impl MimasApp for App {
    fn script_installer(&mut self, install: NativeFnReg) -> &mut Self {
        self.world_mut()
            .get_resource_or_init::<ScriptRegistry>()
            .installers
            .push(install);
        self
    }

    fn script_message<T>(&mut self) -> &mut Self
    where
        T: Message + FromReflect + Typed + GetTypeRegistration,
    {
        self.add_message::<T>().register_type::<T>();
        let mut registry = self.world_mut().get_resource_or_init::<ScriptRegistry>();
        registry.messages.push(TypeId::of::<T>());
        registry.installers.push(api::install_message::<T>);
        self
    }
}

/// The world, lent to natives while a script runs.
pub(crate) type WorldCell = FreezeCell<Freeze![&'freeze mut World]>;

/// What a native registered through [`MimasApp::script_installer`] can reach while a script runs.
pub trait ScriptCtx {
    /// The world, lent for the duration of the closure. Faults outside of a script run.
    fn world<R>(self, f: impl FnOnce(&mut World) -> R) -> Result<R, RtErr>;

    /// The entity whose hook is running. Faults in top-level script code, which has no entity.
    fn entity(self) -> Result<Entity, RtErr>;
}

impl ScriptCtx for Ctx<'_> {
    fn world<R>(self, f: impl FnOnce(&mut World) -> R) -> Result<R, RtErr> {
        self.fixture::<WorldCell>()
            .with_mut(|world| f(world))
            .map_err(|err| {
                RtErr::Custom(match err {
                    FreezeError::Expired => {
                        "the world is only reachable while a script runs".into()
                    }
                    FreezeError::Borrowed => "the world is already borrowed by this native".into(),
                })
            })
    }

    fn entity(self) -> Result<Entity, RtErr> {
        self.fixture::<HookScope>()
            .entity()
            .ok_or_else(|| RtErr::Custom("no script entity outside of a hook".into()))
    }
}
