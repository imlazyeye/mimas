use std::any::TypeId;

use bevy::{
    ecs::reflect::{ReflectComponent, ReflectResource},
    platform::collections::{HashMap, HashSet},
    prelude::World,
    reflect::{TypeInfo, TypeRegistry, std_traits::ReflectDefault},
};

use super::{
    catalog::{Catalog, Reflected},
    kind::{Kind, Members, kind, name},
    stored::Stored,
};

/// Builds a [`Catalog`].
pub(crate) struct Builder<'a> {
    pub catalog: Catalog,
    registry: &'a TypeRegistry,
    world: &'a mut World,
    visiting: HashSet<TypeId>,
}

impl<'a> Builder<'a> {
    pub fn new(registry: &'a TypeRegistry, world: &'a mut World) -> Self {
        Self {
            catalog: Catalog {
                types: HashMap::default(),
                order: Vec::new(),
            },
            registry,
            world,
            visiting: HashSet::default(),
        }
    }

    /// Returns if scripts can use `info`'s type, adding any structs and enums it finds to the
    /// catalog.
    pub fn accept(&mut self, info: &'static TypeInfo) -> bool {
        match kind(info) {
            None => false,
            Some(Kind::Option(inner) | Kind::List(inner)) => self.accept(inner),
            Some(Kind::Type(id)) => {
                if self.catalog.types.contains_key(&id) {
                    return true;
                }
                // recursive types aren't supported
                if !self.visiting.insert(id) {
                    return false;
                }
                let taken = self.take(info);
                self.visiting.remove(&id);
                taken
            }
            Some(_) => true,
        }
    }

    /// Adds a struct or enum to the catalog if scripts can use all of its fields. Field types get
    /// added first.
    fn take(&mut self, info: &'static TypeInfo) -> bool {
        // scripts can't name generic types
        if name(info).contains('<') {
            return false;
        }
        let variants = match info {
            TypeInfo::Enum(variants) => variants.variant_len(),
            _ => 1,
        };
        for index in 0..variants {
            let members =
                Members::of(info, index).expect("the catalog only takes structs and enums");
            for (_, field) in members {
                if !field.is_some_and(|field| self.accept(field)) {
                    return false;
                }
            }
        }

        let id = info.type_id();
        let registration = self.registry.get(id);
        let stored = registration
            .and_then(|registration| registration.data::<ReflectComponent>())
            .map(|reflect| {
                let component = reflect.register_component(self.world);
                Stored {
                    info,
                    reflect: reflect.clone(),
                    id: component,
                    mutable: self
                        .world
                        .components()
                        .get_info(component)
                        .is_none_or(|info| info.mutable()),
                    resource: registration.is_some_and(|registration| {
                        registration.data::<ReflectResource>().is_some()
                    }),
                }
            });
        self.catalog.types.insert(
            id,
            Reflected {
                info,
                stored,
                default: registration
                    .and_then(|registration| registration.data::<ReflectDefault>().cloned()),
            },
        );
        self.catalog.order.push(id);
        true
    }
}

/// Bevy's own types go in the `bevy` module, and everything else goes at the root.
pub(crate) fn module(info: &TypeInfo) -> &'static [&'static str] {
    let from_bevy = info
        .type_path_table()
        .crate_name()
        .is_some_and(|name| BEVY_CRATES.contains(&name));
    if from_bevy { &["bevy"] } else { &[] }
}

/// The crates Bevy is built from (the `bevy_*` dependencies of `bevy_internal` 0.19). Third-party
/// `bevy_*` crates stay at the root. Update this with Bevy.
pub const BEVY_CRATES: &[&str] = &[
    "bevy_a11y",
    "bevy_animation",
    "bevy_anti_alias",
    "bevy_app",
    "bevy_asset",
    "bevy_audio",
    "bevy_camera",
    "bevy_camera_controller",
    "bevy_clipboard",
    "bevy_color",
    "bevy_core_pipeline",
    "bevy_derive",
    "bevy_dev_tools",
    "bevy_diagnostic",
    "bevy_ecs",
    "bevy_feathers",
    "bevy_gilrs",
    "bevy_gizmos",
    "bevy_gizmos_render",
    "bevy_gltf",
    "bevy_image",
    "bevy_input",
    "bevy_input_focus",
    "bevy_light",
    "bevy_log",
    "bevy_material",
    "bevy_math",
    "bevy_mesh",
    "bevy_pbr",
    "bevy_picking",
    "bevy_platform",
    "bevy_post_process",
    "bevy_ptr",
    "bevy_reflect",
    "bevy_remote",
    "bevy_render",
    "bevy_scene",
    "bevy_shader",
    "bevy_solari",
    "bevy_sprite",
    "bevy_sprite_render",
    "bevy_state",
    "bevy_tasks",
    "bevy_text",
    "bevy_time",
    "bevy_transform",
    "bevy_ui",
    "bevy_ui_render",
    "bevy_ui_widgets",
    "bevy_utils",
    "bevy_window",
    "bevy_winit",
    "bevy_world_serialization",
];
