use std::{any::TypeId, sync::Arc};

use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    ecs::{
        message::{MessageCursor, Messages},
        reflect::AppTypeRegistry,
    },
    input::{keyboard::KeyCode, mouse::MouseButton},
    platform::collections::HashSet,
    prelude::*,
};

use crate::{
    bevy::{
        api,
        hooks::{Compiled, ScriptVms},
        plugin::{ScriptRegistry, WorldCell},
        reflect::Catalog,
        script::{MimasScript, render, report},
    },
    vm::Vm,
};

/// Compiles every script again whenever any `.mim` file is added, changed, or removed. Nothing
/// compiles before the scripts folder has loaded, so every module is known by then.
pub(crate) fn compile_scripts(
    world: &mut World,
    mut events: Local<MessageCursor<AssetEvent<MimasScript>>>,
    mut dirty: Local<bool>,
    mut folder_failed: Local<bool>,
) {
    for event in events.read(world.resource::<Messages<AssetEvent<MimasScript>>>()) {
        if matches!(
            event,
            AssetEvent::Added { .. } | AssetEvent::Modified { .. } | AssetEvent::Removed { .. }
        ) {
            *dirty = true;
        }
    }
    if !*dirty {
        return;
    }

    // wait for the scripts folder, so no script compiles before its modules are known
    let folder = world.resource::<ScriptRegistry>().folder.clone();
    if let Some(folder) = folder {
        let server = world.resource::<AssetServer>();
        if !server.is_loaded_with_dependencies(folder.id()) {
            let error = match server.get_load_states(folder.id()) {
                Some((LoadState::Failed(error), ..))
                | Some((.., RecursiveDependencyLoadState::Failed(error))) => error,
                _ => return,
            };
            if !std::mem::replace(&mut *folder_failed, true) {
                error!("the scripts folder didn't load: {error}");
            }
        }
    }
    let Some(mut vms) = world.remove_non_send::<ScriptVms>() else {
        return;
    };
    *dirty = false;

    // modules compile into every script, and never run on their own
    let (modules, scripts): (Vec<_>, Vec<_>) = world
        .resource::<Assets<MimasScript>>()
        .iter()
        .map(|(id, script)| (id, script.clone()))
        .partition(|(_, script)| script.is_module());
    vms.compiled
        .retain(|id, _| scripts.iter().any(|(script, _)| script == id));

    let registry = world.resource::<ScriptRegistry>();
    let installers = registry.installers.clone();
    let extra: Vec<TypeId> = [TypeId::of::<KeyCode>(), TypeId::of::<MouseButton>()]
        .into_iter()
        .chain(registry.messages.iter().copied())
        .collect();
    let catalog = Arc::new(Catalog::new(
        &world.resource::<AppTypeRegistry>().read(),
        &extra,
    ));

    // a broken module fails every script the same way, and that should read once
    let mut reported = HashSet::new();
    for (id, script) in scripts {
        let files: Vec<(&str, &str)> = std::iter::once(&script)
            .chain(modules.iter().map(|(_, module)| module))
            .map(|file| (file.path.as_str(), file.source.as_str()))
            .collect();
        let compiled = Vm::compile_files(&files, |api| {
            crate::library::std(api);
            api::install(api, &catalog);
            for install in &installers {
                install(api);
            }
        })
        .map_err(|err| err.0)
        .and_then(|mut vm| {
            // the top-level statements run now, with the world in reach
            vm.fixture::<WorldCell>().freeze(world, || vm.run())?;
            let handle = world
                .resource_mut::<Assets<MimasScript>>()
                .get_strong_handle(id);
            Compiled::new(vm, catalog.clone(), handle)
        });
        // a failed reload keeps the last good version running
        match compiled {
            Ok(compiled) => {
                vms.compiled.insert(id, compiled);
            }
            Err(err) => {
                if reported.insert(render(&err)) {
                    report(world, id, None, &err);
                }
            }
        }
    }
    world.insert_non_send(vms);
}
