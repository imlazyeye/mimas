use std::sync::Arc;

use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    ecs::message::{MessageCursor, Messages},
    platform::collections::HashSet,
    prelude::*,
};

use crate::{
    api,
    hooks::{ScriptVm, ScriptVms},
    plugin::{ScriptRegistry, WorldCell},
    reflect::Catalog,
    script::{MimasScript, render, report},
};
use vm::Vm;

/// Recompiles every script when any `.mim` file changes, once the scripts folder has loaded.
pub(crate) fn compile_scripts(
    world: &mut World,
    mut events: Local<MessageCursor<AssetEvent<MimasScript>>>,
) {
    // `count` instead of `any`, to drain the cursor
    let changed = events
        .read(world.resource::<Messages<AssetEvent<MimasScript>>>())
        .filter(|event| {
            matches!(
                event,
                AssetEvent::Added { .. } | AssetEvent::Modified { .. } | AssetEvent::Removed { .. }
            )
        })
        .count();
    if changed == 0 {
        return;
    }

    // don't compile until every module has loaded
    let folder = world.resource::<ScriptRegistry>().folder.clone();
    if let Some(folder) = folder {
        let server = world.resource::<AssetServer>();
        if !server.is_loaded_with_dependencies(folder.id()) {
            let error = match server.get_load_states(folder.id()) {
                Some((LoadState::Failed(error), ..))
                | Some((.., RecursiveDependencyLoadState::Failed(error))) => error,
                _ => return,
            };
            error!("the scripts folder didn't load: {error}");
        }
    }
    let Some(mut vms) = world.remove_non_send::<ScriptVms>() else {
        return;
    };

    // modules get compiled into every script
    let (modules, scripts): (Vec<_>, Vec<_>) = world
        .resource::<Assets<MimasScript>>()
        .iter()
        .map(|(id, script)| (id, script.clone()))
        .partition(|(_, script)| script.is_module());
    vms.by_asset
        .retain(|id, _| scripts.iter().any(|(script, _)| script == id));

    let registry = world.resource::<ScriptRegistry>();
    let installers = registry.installers.clone();
    let types = registry.types.clone();
    let catalog = Arc::new(Catalog::new(world, &types));

    // a broken module fails every script with the same error
    let mut reported = HashSet::new();
    for (id, script) in scripts {
        let files: Vec<(&str, &str)> = std::iter::once(&script)
            .chain(modules.iter().map(|(_, module)| module))
            .map(|file| (file.path.as_str(), file.source.as_str()))
            .collect();
        let vm = Vm::compile_files(&files, |api| {
            library::std(api);
            api::install(api, &catalog);
            for install in &installers {
                install(api);
            }
        })
        .map_err(|err| err.0)
        .and_then(|mut vm| {
            vm.fixture::<WorldCell>().freeze(world, || vm.run())?;
            Ok(ScriptVm::new(vm, catalog.clone()))
        });
        // a failed reload keeps the last good version running
        match vm {
            Ok(vm) => {
                vms.by_asset.insert(id, vm);
            }
            Err(err) => {
                let message = render(&err);
                if reported.insert(message.clone()) {
                    report(world, id, None, &err, message);
                }
            }
        }
    }
    world.insert_non_send(vms);
}
