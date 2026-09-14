use std::{
    any::TypeId,
    cell::{Cell, RefCell},
    sync::Arc,
};

use bevy::{
    ecs::{lifecycle::RemovedComponents, system::SystemState},
    platform::collections::HashMap,
    prelude::*,
};
use miette::miette;

use crate::{
    plugin::WorldCell,
    reflect::Catalog,
    script::{MimasScript, Script, render, report},
    writeback::HookScope,
};
use vm::{Args, Ctx, FixtureRef, Registry, RtErr, Stashed, Ty, Val, Vm};

/// The hooks a script can register functions to.
#[derive(Clone, Copy)]
pub(crate) enum Hook {
    Start,
    Update,
    FixedUpdate,
    Stop,
}

impl Hook {
    pub const ALL: [Hook; 4] = [Hook::Start, Hook::Update, Hook::FixedUpdate, Hook::Stop];

    pub fn name(self) -> &'static str {
        match self {
            Hook::Start => "start",
            Hook::Update => "update",
            Hook::FixedUpdate => "fixed_update",
            Hook::Stop => "stop",
        }
    }
}

/// One hook parameter (the component or resource that fills it).
struct Param {
    type_id: TypeId,
    resource: bool,
    optional: bool,
}

/// A registered hook (the function value to call, what fills its parameters, and if it's
/// reported a fault yet).
struct HookFn {
    f: Stashed,
    params: Vec<Param>,
    reported: Cell<bool>,
}

/// What the `bevy` module's registration natives have taken during this compile, in the order the
/// script registered them.
#[derive(Default)]
pub(crate) struct Registrations(RefCell<Vec<(Hook, HookFn)>>);

/// Registers `f` to run at `hook`, working out what fills each of its parameters first. `f` has to
/// be a fn or a closure, and each parameter has to be a component or a resource (unless they are
/// defaults the script provided).
pub(crate) fn register<'gc>(
    ctx: Ctx<'gc>,
    catalog: &Catalog,
    hook: Hook,
    f: Val<'gc>,
) -> Result<(), RtErr> {
    if ctx.fixture::<HookScope>().entity().is_ok() {
        return Err(RtErr::Custom(format!(
            "`bevy::{}` registers in top-level code, which has already run",
            hook.name()
        )));
    }
    let Some(header) = f.signature(ctx) else {
        return Err(RtErr::InvalidArgument(format!(
            "`bevy::{}` takes a function, but a `{}` was passed",
            hook.name(),
            ctx.display(f)
        )));
    };
    let mut params = Vec::new();
    for (i, param) in header.parameters.iter().enumerate() {
        let name = param.name.as_deref().unwrap_or("_");
        let (wanted, optional) = match &param.ty {
            Ty::Option(inner) => (inner.as_ref(), true),
            ty => (ty, false),
        };
        // compare adts instead of the whole `Ty`, because a method's `self` shows up as `Self`
        let found = wanted.as_adt().and_then(|adt| {
            catalog.stored().find(|(id, _)| {
                ctx.binding(*id)
                    .is_some_and(|binding| binding.adt_id == *adt)
            })
        });
        let Some((type_id, stored)) = found else {
            if header.parameters[i..].iter().all(|param| param.has_default) {
                break;
            }
            return Err(RtErr::Custom(format!(
                "`{}` asks for `{name}: {}`, which isn't a component or resource",
                hook.name(),
                param.ty
            )));
        };
        params.push(Param {
            type_id,
            resource: stored.resource,
            optional,
        });
    }

    ctx.fixture::<Registrations>().0.borrow_mut().push((
        hook,
        HookFn {
            f: ctx.stash(f),
            params,
            reported: Cell::default(),
        },
    ));
    Ok(())
}

/// The arguments for one hook call.
struct HookArgs<'a> {
    params: &'a [Param],
    entity: Entity,
    scope: &'a HookScope,
    catalog: &'a Catalog,
}

impl Args for HookArgs<'_> {
    fn tys(&self, _: &Registry) -> Vec<Option<Ty>> {
        vec![None; self.params.len()]
    }

    fn into_values<'gc>(self, ctx: Ctx<'gc>) -> Vec<Val<'gc>> {
        self.params
            .iter()
            .map(|param| {
                let entity = (!param.resource).then_some(self.entity);
                self.scope
                    .lend(ctx, self.catalog, entity, param.type_id)
                    .expect("the world is lent for the whole hook")
                    .unwrap_or(Val::Null)
            })
            .collect()
    }
}

/// Every compiled script. Vms aren't `Send`.
#[derive(Default)]
pub(crate) struct ScriptVms {
    pub by_asset: HashMap<AssetId<MimasScript>, ScriptVm>,
    // the asset each entity started with, to catch when its script gets swapped
    started: HashMap<Entity, AssetId<MimasScript>>,
}

impl ScriptVms {
    fn call(
        &mut self,
        world: &mut World,
        asset: AssetId<MimasScript>,
        entity: Entity,
        hook: Hook,
    ) -> bool {
        match self.by_asset.get_mut(&asset) {
            Some(vm) => vm.call(world, asset, entity, hook),
            None => false,
        }
    }
}

/// A compiled script and the hooks it registered.
pub(crate) struct ScriptVm {
    vm: Vm,
    world: FixtureRef<WorldCell>,
    scope: FixtureRef<HookScope>,
    catalog: Arc<Catalog>,
    hooks: [Vec<HookFn>; 4],
}

impl ScriptVm {
    /// Takes a Vm after its top-level code has run.
    pub fn new(vm: Vm, catalog: Arc<Catalog>) -> Self {
        let mut hooks: [Vec<HookFn>; 4] = Default::default();
        for (hook, f) in vm.fixture::<Registrations>().0.take() {
            hooks[hook as usize].push(f);
        }
        Self {
            world: vm.fixture(),
            scope: vm.fixture(),
            catalog,
            vm,
            hooks,
        }
    }

    /// Runs every function registered at `hook`, and returns if they all ran without a fault. Each
    /// function only reports its first fault.
    fn call(
        &mut self,
        world: &mut World,
        script: AssetId<MimasScript>,
        entity: Entity,
        hook: Hook,
    ) -> bool {
        let mut all_ran = true;
        for HookFn {
            f,
            params,
            reported,
        } in &self.hooks[hook as usize]
        {
            let missing = params.iter().find(|param| {
                !param.optional && !self.catalog.get(param.type_id).has(world, entity)
            });
            if let Some(param) = missing {
                all_ran = false;
                if !reported.replace(true) {
                    let err = miette!(
                        "`{}` needs a `{}` that {entity} doesn't have",
                        hook.name(),
                        self.catalog.get(param.type_id).name()
                    );
                    report(world, script, Some(entity), &err, render(&err));
                }
                continue;
            }
            self.scope.begin(entity);
            let (scope, catalog) = (&*self.scope, &*self.catalog);
            let args = HookArgs {
                params,
                entity,
                scope,
                catalog,
            };
            let ran = self.world.freeze(world, || {
                // write everything back before the gc gets a chance to run
                self.vm
                    .call_value_then::<(), _>(f, args, |ctx| scope.write_back(ctx, catalog))
            });
            self.scope.end();
            if let Err(err) = ran {
                all_ran = false;
                if !reported.replace(true) {
                    report(world, script, Some(entity), &err, render(&err));
                }
            }
        }
        all_ran
    }
}

/// The asset an entity's `Script` currently points at.
fn current(world: &World, entity: Entity) -> Option<AssetId<MimasScript>> {
    world.get::<Script>(entity).map(|script| script.0.id())
}

/// Runs `stop` for scripts that went away or got swapped, and `start` for new ones.
pub(crate) fn lifecycle(
    world: &mut World,
    removed: &mut SystemState<RemovedComponents<Script>>,
    scripts: &mut QueryState<(Entity, &Script)>,
) {
    let Some(mut vms) = world.remove_non_send::<ScriptVms>() else {
        return;
    };

    let gone: Vec<Entity> = removed
        .get_mut(world)
        .map(|mut removed| removed.read().collect())
        .unwrap_or_default();
    for entity in gone {
        if let Some(asset) = vms.started.remove(&entity) {
            vms.call(world, asset, entity, Hook::Stop);
        }
    }

    let scripted: Vec<(Entity, AssetId<MimasScript>)> = scripts
        .iter(world)
        .map(|(entity, script)| (entity, script.0.id()))
        .filter(|(entity, asset)| {
            vms.started.get(entity) != Some(asset) && vms.by_asset.contains_key(asset)
        })
        .collect();
    for (entity, asset) in scripted {
        // an earlier `start` might have changed this entity's script
        if current(world, entity) != Some(asset) {
            continue;
        }
        if let Some(old) = vms.started.remove(&entity) {
            vms.call(world, old, entity, Hook::Stop);
        }
        // if `start` doesn't fully run, we try again next frame
        if vms.call(world, asset, entity, Hook::Start) {
            vms.started.insert(entity, asset);
        }
    }

    world.insert_non_send(vms);
}

/// Runs `update` for every started script.
pub(crate) fn update(world: &mut World, scripts: &mut QueryState<(Entity, &Script)>) {
    run(world, scripts, Hook::Update);
}

/// Runs `fixed_update` for every started script.
pub(crate) fn fixed_update(world: &mut World, scripts: &mut QueryState<(Entity, &Script)>) {
    run(world, scripts, Hook::FixedUpdate);
}

fn run(world: &mut World, scripts: &mut QueryState<(Entity, &Script)>, hook: Hook) {
    let Some(mut vms) = world.remove_non_send::<ScriptVms>() else {
        return;
    };
    let started: Vec<(Entity, AssetId<MimasScript>)> = scripts
        .iter(world)
        .map(|(entity, script)| (entity, script.0.id()))
        .filter(|(entity, asset)| vms.started.get(entity) == Some(asset))
        .collect();
    for (entity, asset) in started {
        // an earlier hook might have changed this entity's script
        if current(world, entity) == Some(asset) {
            vms.call(world, asset, entity, hook);
        }
    }
    world.insert_non_send(vms);
}
