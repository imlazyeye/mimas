use std::{any::TypeId, sync::Arc};

use bevy::{
    ecs::{lifecycle::RemovedComponents, system::SystemState},
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use miette::{Report, miette};

use crate::{
    bevy::{
        api::lend,
        plugin::WorldCell,
        reflect::Catalog,
        script::{MimasScript, Script, report},
        writeback::HookScope,
    },
    vm::{Args, Ctx, FixtureRef, Function, Registry, Ty, Val, Vm},
};

/// The functions a script may declare at its root, each optional.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Hook {
    Start,
    Update,
    FixedUpdate,
    Stop,
}

impl Hook {
    const ALL: [Hook; 4] = [Hook::Start, Hook::Update, Hook::FixedUpdate, Hook::Stop];

    fn name(self) -> &'static str {
        match self {
            Hook::Start => "start",
            Hook::Update => "update",
            Hook::FixedUpdate => "fixed_update",
            Hook::Stop => "stop",
        }
    }
}

/// One hook parameter: its declared type and the component or resource that fills it.
struct Param {
    ty: Ty,
    type_id: TypeId,
    resource: bool,
    optional: bool,
}

/// A hook resolved at load: the function to call and what fills its parameters.
struct HookFn {
    f: Function,
    params: Vec<Param>,
}

/// The arguments of one hook call, lent from the world as the call starts.
struct HookArgs<'a> {
    params: &'a [Param],
    entity: Entity,
}

impl Args for HookArgs<'_> {
    fn tys(&self, _: &Registry) -> Vec<Option<Ty>> {
        self.params
            .iter()
            .map(|param| Some(param.ty.clone()))
            .collect()
    }

    fn into_values<'gc>(self, ctx: Ctx<'gc>) -> Vec<Val<'gc>> {
        self.params
            .iter()
            .map(|param| {
                let entity = (!param.resource).then_some(self.entity);
                lend(ctx, param.type_id, entity)
                    .expect("the world is lent for the whole hook")
                    .unwrap_or(Val::Null)
            })
            .collect()
    }
}

/// The compiled scripts. A Vm isn't `Send`, so they live outside the ECS's `Send` world.
#[derive(Default)]
pub(crate) struct ScriptVms {
    pub compiled: HashMap<AssetId<MimasScript>, Compiled>,
    // which asset each entity's `start` ran for, which changes when the handle is swapped
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
        match self.compiled.get_mut(&asset) {
            Some(compiled) => compiled.call(world, asset, entity, hook),
            None => false,
        }
    }
}

/// One script's Vm, with its hooks resolved once at load.
pub(crate) struct Compiled {
    vm: Vm,
    world: FixtureRef<WorldCell>,
    scope: FixtureRef<HookScope>,
    catalog: Arc<Catalog>,
    // keeps the asset loaded while its Vm is, since entities alone would drop it between uses
    _handle: Option<Handle<MimasScript>>,
    hooks: [Option<HookFn>; 4],
    // hook and entity pairs that have faulted since this compile, each reported once
    reported: HashSet<(Hook, Entity)>,
}

impl Compiled {
    pub fn new(
        vm: Vm,
        catalog: Arc<Catalog>,
        handle: Option<Handle<MimasScript>>,
    ) -> Result<Self, Report> {
        let mut hooks = [None, None, None, None];
        for hook in Hook::ALL {
            let Some(f) = vm.root().function(hook.name()) else {
                continue;
            };
            // fill every parameter from a component or resource of its type, or leave a
            // defaulted tail to the script
            let mut params = Vec::new();
            for (i, param) in f.header.parameters.iter().enumerate() {
                let name = param.name.as_deref().unwrap_or("_");
                let (wanted, optional) = match &param.ty {
                    Ty::Option(inner) => (inner.as_ref(), true),
                    ty => (ty, false),
                };
                let stored = catalog
                    .stored()
                    .find(|(id, _)| Catalog::ty(vm.registry(), *id).as_ref() == Some(wanted));
                let Some((type_id, reflected)) = stored else {
                    if f.defaults[i..].iter().all(Option::is_some) {
                        break;
                    }
                    return Err(miette!(
                        "`{}` asks for `{name}: {}`, which isn't a component or resource",
                        hook.name(),
                        param.ty
                    ));
                };
                if hook == Hook::Stop && !reflected.resource {
                    return Err(miette!(
                        "`stop` runs after its entity is gone, so it can't take `{name}: {}`",
                        param.ty
                    ));
                }
                params.push(Param {
                    ty: param.ty.clone(),
                    type_id,
                    resource: reflected.resource,
                    optional,
                });
            }
            let given: Vec<Option<Ty>> =
                params.iter().map(|param| Some(param.ty.clone())).collect();
            f.check(hook.name(), &given, Some(&Ty::Unit))?;
            hooks[hook as usize] = Some(HookFn {
                f: f.clone(),
                params,
            });
        }
        Ok(Self {
            world: vm.fixture(),
            scope: vm.fixture(),
            catalog,
            vm,
            _handle: handle,
            hooks,
            reported: HashSet::default(),
        })
    }

    /// Whether the hook ran without a fault. A hook the script doesn't declare counts as run.
    fn call(
        &mut self,
        world: &mut World,
        script: AssetId<MimasScript>,
        entity: Entity,
        hook: Hook,
    ) -> bool {
        let Some(HookFn { f, params }) = &self.hooks[hook as usize] else {
            return true;
        };
        let missing = params
            .iter()
            .find(|param| !param.optional && !self.catalog.has(world, param.type_id, entity));
        let ran = match missing {
            Some(param) => Err(miette!(
                "`{}` needs a `{}` that {entity} doesn't have",
                hook.name(),
                param.ty
            )),
            None => {
                self.scope.begin(entity);
                let (scope, catalog) = (&self.scope, &self.catalog);
                let args = HookArgs { params, entity };
                let ran = self.world.freeze(world, || {
                    // everything lent out goes back into the world before anything is collected
                    self.vm
                        .call_function::<(), _>(f, args, |ctx| scope.write_back(ctx, catalog))
                });
                self.scope.end();
                ran.map(|_| ())
            }
        };
        if let Err(err) = &ran
            && self.reported.insert((hook, entity))
        {
            report(world, script, Some(entity), err);
        }
        // a stopped entity is forgotten, so a reused id reports afresh
        if hook == Hook::Stop {
            self.reported.retain(|(_, reported)| *reported != entity);
        }
        ran.is_ok()
    }
}

/// The system that runs `hook` for every scripted entity: `stop` first for the ones whose script
/// went away, `start` for the new ones, then the hook itself.
pub(crate) fn run(hook: Hook) -> impl FnMut(&mut World) {
    let mut scripts: Option<QueryState<(Entity, &'static Script)>> = None;
    let mut removed: Option<SystemState<RemovedComponents<'static, 'static, Script>>> = None;
    move |world| {
        let Some(mut vms) = world.remove_non_send::<ScriptVms>() else {
            return;
        };

        let gone: Vec<Entity> = removed
            .get_or_insert_with(|| SystemState::new(world))
            .get_mut(world)
            .map(|mut removed| removed.read().collect())
            .unwrap_or_default();
        for entity in gone {
            if let Some(asset) = vms.started.remove(&entity) {
                vms.call(world, asset, entity, Hook::Stop);
            }
        }

        let running: Vec<(Entity, AssetId<MimasScript>)> = scripts
            .get_or_insert_with(|| world.query())
            .iter(world)
            .map(|(entity, script)| (entity, script.0.id()))
            .collect();
        for (entity, asset) in running {
            // still loading, or despawned by an earlier script this pass
            if !vms.compiled.contains_key(&asset) || world.get::<Script>(entity).is_none() {
                continue;
            }
            if vms.started.get(&entity) != Some(&asset) {
                if let Some(old) = vms.started.remove(&entity) {
                    vms.call(world, old, entity, Hook::Stop);
                }
                // a start that couldn't run is retried next pass, and one that despawned the
                // entity is the end of it
                if !vms.call(world, asset, entity, Hook::Start)
                    || world.get::<Script>(entity).is_none()
                {
                    continue;
                }
                vms.started.insert(entity, asset);
            }
            vms.call(world, asset, entity, hook);
        }

        world.insert_non_send(vms);
    }
}
