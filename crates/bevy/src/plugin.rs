use std::any::TypeId;

use bevy::{
    asset::LoadedFolder,
    ecs::message::Message,
    prelude::*,
    reflect::{FromReflect, GetTypeRegistration, Typed},
};

use crate::{api, compile, hooks, script, writeback::HookScope};
use vm::{
    Ctx, RtErr,
    api::NativeFnReg,
    freeze::{Freeze, FreezeCell},
};

/// Runs `.mim` files as scripts. A file that declares a module gets compiled into every other
/// script instead of running on its own.
pub struct MimasPlugin {
    /// Paths under `assets/` to load at startup.
    scripts: Vec<String>,
    /// A folder under `assets/` to load whole at startup.
    folder: Option<String>,
}

impl MimasPlugin {
    /// Loads each of `scripts` and nothing else.
    pub fn new(scripts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            scripts: scripts.into_iter().map(Into::into).collect(),
            folder: None,
        }
    }

    /// Loads every `.mim` file in a folder relative to `assets/`.
    pub fn folder(folder: impl Into<String>) -> Self {
        Self {
            scripts: vec![],
            folder: Some(folder.into()),
        }
    }
}

impl Default for MimasPlugin {
    /// Loads the `assets/scripts` folder.
    fn default() -> Self {
        Self::folder("scripts")
    }
}

/// The systems that compile and run scripts.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MimasSystems;

impl Plugin for MimasPlugin {
    fn build(&self, app: &mut App) {
        // the input stuff is strange because its not on some platforms
        app.register_type::<KeyCode>()
            .register_type::<MouseButton>()
            .init_asset::<script::MimasScript>()
            .init_asset_loader::<script::MimasScriptLoader>()
            .add_message::<script::ScriptError>()
            .init_resource::<ScriptRegistry>()
            .add_systems(
                PreUpdate,
                (compile::compile_scripts, hooks::lifecycle)
                    .chain()
                    .in_set(MimasSystems),
            )
            .add_systems(FixedUpdate, hooks::fixed_update.in_set(MimasSystems))
            .add_systems(Update, hooks::update.in_set(MimasSystems));
        // the loader needs to exist before anything loads
        let assets = app.world().resource::<AssetServer>().clone();
        let folder = self.folder.as_ref().map(|path| assets.load_folder(path));
        let scripts = self
            .scripts
            .iter()
            .map(|path| assets.load::<script::MimasScript>(path))
            .collect();
        let mut registry = app.world_mut().resource_mut::<ScriptRegistry>();
        registry.folder = folder;
        registry.scripts = scripts;
        registry
            .types
            .extend([TypeId::of::<KeyCode>(), TypeId::of::<MouseButton>()]);
        app.world_mut().insert_non_send(hooks::ScriptVms::default());
    }
}

/// Anything the app registered for scripts through [`MimasApp`].
#[derive(Resource, Default)]
pub(crate) struct ScriptRegistry {
    pub installers: Vec<NativeFnReg>,
    /// Types to include that aren't components or resources (like messages).
    pub types: Vec<TypeId>,
    /// Handles kept so whatever the plugin loaded stays loaded.
    pub folder: Option<Handle<LoadedFolder>>,
    pub scripts: Vec<Handle<script::MimasScript>>,
}

/// Extra setup for scripts, on top of what reflection finds.
pub trait MimasApp {
    /// Runs `install` on every script's `Api`, after the plugin's own natives. Natives use
    /// [`ScriptCtx`] to get at the world.
    fn script_installer(&mut self, install: NativeFnReg) -> &mut Self;

    /// Lets scripts `read` and `write` the message `T`. Unlike components, messages have to be
    /// registered by hand (Bevy can only read them through their concrete type).
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
        registry.types.push(TypeId::of::<T>());
        registry.installers.push(api::install_message::<T>);
        self
    }
}

/// Holds the world while a script is running.
pub(crate) type WorldCell = FreezeCell<Freeze![&'freeze mut World]>;

/// Access to Bevy from inside natives added with [`MimasApp::script_installer`].
pub trait ScriptCtx {
    /// Gives `f` the world. Panics outside of a script run, or if called again from inside `f`.
    fn world<R>(self, f: impl FnOnce(&mut World) -> R) -> R;

    /// The entity whose hook is running (an error outside of a hook).
    fn entity(self) -> Result<Entity, RtErr>;
}

impl ScriptCtx for Ctx<'_> {
    fn world<R>(self, f: impl FnOnce(&mut World) -> R) -> R {
        self.fixture::<WorldCell>()
            .with_mut(|world| f(world))
            .expect("the world is lent to one native at a time, while a script runs")
    }

    fn entity(self) -> Result<Entity, RtErr> {
        self.fixture::<HookScope>().entity()
    }
}
