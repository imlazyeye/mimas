use api::{
    AdtBinding, ApiAdt, ApiAdtKind, ApiConstant, ApiFunction, ApiMethod, ApiVariant, Intrinsic,
    Library, NativeId,
};
use shared::{Literal, Ty};

use crate::{
    RtErr,
    adt::MimasAdt,
    conversion::{IntoNativeResult, MimasType},
    heap::Ctx,
    native::{NativeRef, make_native},
};

/// A single native-registration request, emitted by the `#[mimas]` attribute macro and
/// gathered at install time through the [`inventory`](crate::inventory) crate.
///
/// This is what allows the macro to both convert _and_ import your item. The key issue is that
/// there's obviously no way to find and mutate your VM at this stage, so instead we use the
/// inventory crate to collect a series of functions we generate per-impl in the macro that can be
/// called later to do the installation.
///
/// inventory makes **no guarantee** about the order in which submissions are visited. That
/// collides with the load-order rule in [`Api::add_adt`]: a Rust ADT must be registered before
/// any fn whose mimas signature mentions it. `phase` lets `library::std` run a deterministic
/// two-pass sweep -- ADTs ([`Self::PHASE_ADT`]) first, everything else ([`Self::PHASE_FN`])
/// second -- so that rule holds regardless of inventory's iteration order.
pub struct MimasReg {
    pub phase: u8,
    pub register: NativeFnReg,
}

impl MimasReg {
    pub const PHASE_ADT: u8 = 0;
    pub const PHASE_FN: u8 = 1;
}

inventory::collect!(MimasReg);

/// A doc-comment harvested from a `#[native]` / `#[mimas]` item, submitted via [`inventory`] and
/// keyed by the item's full Rust path (`concat!(module_path!(), "::", <ident>)`). The install path
/// joins these onto the [`ApiFunction`]/[`ApiMethod`] it builds by matching the path against
/// [`std::any::type_name_of_val`] of the registered fn, so the receiver/module -- known only at the
/// `api.add_*` call site, not the macro -- never has to travel with the doc.
///
/// Like every inventory registry this is subject to the link-pruning footgun (a submission in an
/// unreferenced object file can be dropped under `codegen-units > 1`). That only affects the
/// doc-generation build, which we own and can pin to `codegen-units = 1`; missing a doc degrades to
/// an empty string, never a wrong signature.
pub struct NativeDoc {
    pub path: &'static str,
    pub doc: &'static str,
}

inventory::collect!(NativeDoc);

pub type NativeFnReg = for<'a, 'gc> fn(&mut Api<'a, 'gc>);

pub struct Api<'a, 'gc> {
    pub ctx: Ctx<'gc>,
    pub library: &'a mut Library<()>,
}

/// Look up the doc-comment for a native by its full Rust path (`type_name_of_val(&f)`). Empty when
/// the item carried no `///` -- or when the submission was pruned (see [`NativeDoc`]).
fn doc_for(path: &str) -> String {
    inventory::iter::<NativeDoc>
        .into_iter()
        .find(|d| d.path == path)
        .map(|d| d.doc.to_string())
        .unwrap_or_default()
}

impl<'a, 'gc> Api<'a, 'gc> {
    pub fn add<F, Marker>(&mut self, f: F)
    where
        F: IntoFn<'gc, Marker>,
    {
        let name = short_name_of_val(&f);
        f.install(self, name, Vec::new());
    }

    pub fn add_named<F, Marker>(&mut self, name: impl Into<String>, f: F)
    where
        F: IntoFn<'gc, Marker>,
    {
        f.install(self, name.into(), Vec::new());
    }

    pub fn add_method<F, Marker>(&mut self, f: F) -> NativeId
    where
        F: IntoMethod<'gc, Marker>,
    {
        let name = short_name_of_val(&f);
        f.install_method(self, name)
    }

    pub fn add_method_named<F, Marker>(&mut self, name: impl Into<String>, f: F)
    where
        F: IntoMethod<'gc, Marker>,
    {
        f.install_method(self, name.into());
    }

    /// Associated functions don't carry a `self` arg the macro can introspect, so the receiver
    /// `Ty` (the builtin's namespace -- `Ty::Int`, `Ty::Bool`, etc.) is passed explicitly.
    pub fn add_assoc<F, Marker>(&mut self, recv_ty: Ty, f: F)
    where
        F: IntoFn<'gc, Marker>,
    {
        let name = short_name_of_val(&f);
        f.install_assoc(self, recv_ty, name);
    }

    /// The mimas `Ty` of a registered Rust type -- the receiver arg for [`Self::add_assoc`].
    /// Panics if `add_adt::<T>()` hasn't run yet.
    pub fn ty_of<T: MimasType<'gc>>(&self) -> Ty {
        T::mimas_ty(self.library.registry()).expect("ty_of requires the type be registered first")
    }

    /// [`Self::add_assoc`] with the receiver resolved from a registered adt -- what `#[mimas]
    /// impl` uses for `self`-less fns, whose generated shim name isn't the mimas-facing one.
    pub fn add_assoc_of<T, F, Marker>(&mut self, name: impl Into<String>, f: F)
    where
        T: MimasType<'gc>,
        F: IntoFn<'gc, Marker>,
    {
        let recv_ty =
            T::mimas_ty(self.library.registry()).expect("assoc receiver must be a registered adt");
        f.install_assoc(self, recv_ty, name.into());
    }

    pub fn mark_intrinsic(&mut self, nid: NativeId, i: Intrinsic) {
        self.library.mark_instrinsic(nid, i);
    }

    // must run before any `add(fn)` whose signature mentions `T`, or any add_adt::<U>
    // whose fields reference `T` -- field types resolve eagerly through the registry.
    //
    // todo, make this order independent
    pub fn add_adt<T: MimasAdt>(&mut self) {
        self.add_adt_in::<T>(None);
    }

    fn add_adt_in<T: MimasAdt>(&mut self, module: Option<Vec<String>>) {
        let adt_id = self.library.registry_mut().alloc();
        // pre-bind T so that it can be self-referential
        self.library.registry_mut().bind::<T>(AdtBinding {
            adt_id,
            variant_layout_ids: Vec::new(),
        });
        let desc = T::descriptor(self.library.registry());
        let variant_layout_ids: Vec<_> = match desc.kind {
            ApiAdtKind::Enum => desc
                .variants
                .iter()
                .map(|_| self.library.registry_mut().alloc())
                .collect(),
            ApiAdtKind::Struct => vec![adt_id],
        };
        self.library.registry_mut().bind::<T>(AdtBinding {
            adt_id,
            variant_layout_ids: variant_layout_ids.clone(),
        });

        let variants = desc
            .variants
            .into_iter()
            .zip(variant_layout_ids.iter().copied())
            .map(|(shape, layout_id)| ApiVariant {
                name: shape.name,
                layout_id,
                doc: shape.doc.to_string(),
                fields: shape.fields,
            })
            .collect();

        let module =
            module.unwrap_or_else(|| desc.module.iter().map(|s| (*s).to_string()).collect());
        self.library.push_adt(ApiAdt {
            name: desc.name.to_string(),
            module,
            kind: desc.kind,
            adt_id,
            doc: desc.doc.to_string(),
            variants,
        });

        let binding = AdtBinding {
            adt_id,
            variant_layout_ids,
        };
        let mut bindings = self.ctx.state().mimas_bindings.borrow_mut(&self.ctx);
        bindings.0.insert(std::any::TypeId::of::<T>(), binding);
    }

    /// Prelude-level constant (no module). The module-scoped counterpart is
    /// [`ModuleApi::constant`].
    pub fn constant(
        &mut self,
        name: impl Into<String>,
        ty: Ty,
        value: Literal,
        doc: impl Into<String>,
    ) {
        self.library.constant(ApiConstant {
            name: name.into(),
            module: Vec::new(),
            recv_ty: None,
            ty,
            value,
            doc: doc.into(),
        });
    }

    /// An associated constant (`Player::MAX_HEALTH`) -- what `#[mimas] impl` uses for consts
    /// in the block.
    pub fn assoc_constant(
        &mut self,
        recv_ty: Ty,
        name: impl Into<String>,
        ty: Ty,
        value: Literal,
        doc: impl Into<String>,
    ) {
        self.library.constant(ApiConstant {
            name: name.into(),
            module: Vec::new(),
            recv_ty: Some(recv_ty),
            ty,
            value,
            doc: doc.into(),
        });
    }

    pub fn module<'b>(&'b mut self, path: impl Into<String>) -> ModuleApi<'b, 'a, 'gc> {
        let path: Vec<String> = path.into().split("::").map(String::from).collect();
        ModuleApi { parent: self, path }
    }

    fn store_native(&self, id: NativeId, native: NativeRef<'gc>) {
        let mut natives = self.ctx.state().natives.borrow_mut(&self.ctx);
        if natives.len() <= id.index() {
            natives.resize(id.index() + 1, None);
        }
        natives[id.index()] = Some(native);
    }
}

pub struct ModuleApi<'b, 'a, 'gc> {
    parent: &'b mut Api<'a, 'gc>,
    path: Vec<String>,
}

impl<'b, 'a, 'gc> ModuleApi<'b, 'a, 'gc> {
    pub fn add<F, Marker>(&mut self, f: F)
    where
        F: IntoFn<'gc, Marker>,
    {
        let name = short_name_of_val(&f);
        f.install(self.parent, name, self.path.clone());
    }

    pub fn add_named<F, Marker>(&mut self, name: impl Into<String>, f: F)
    where
        F: IntoFn<'gc, Marker>,
    {
        f.install(self.parent, name.into(), self.path.clone());
    }

    pub fn add_adt<T: MimasAdt>(&mut self) {
        self.parent.add_adt_in::<T>(Some(self.path.clone()));
    }

    pub fn constant(
        &mut self,
        name: impl Into<String>,
        ty: Ty,
        value: Literal,
        doc: impl Into<String>,
    ) {
        self.parent.library.constant(ApiConstant {
            name: name.into(),
            module: self.path.clone(),
            recv_ty: None,
            ty,
            value,
            doc: doc.into(),
        });
    }
}

// `Marker` is a phantom tuple (e.g. `(A, B)`) that disambiguates which arity-impl matches
// a given fn item. Rust picks the impl whose Fn-signature aligns with F's; without the
// marker type param the per-arity impls would all collide on `impl IntoFn<'gc> for F`.

pub trait IntoFn<'gc, Marker>: Copy + 'static {
    fn install(self, api: &mut Api<'_, 'gc>, name: String, module: Vec<String>) -> NativeId;

    fn install_assoc(self, api: &mut Api<'_, 'gc>, recv_ty: Ty, name: String);
}

pub trait IntoMethod<'gc, Marker>: Copy + 'static {
    fn install_method(self, api: &mut Api<'_, 'gc>, name: String) -> NativeId;
}

macro_rules! impl_into_fn {
    ($($arg:ident),*) => {
        impl<'gc, F, R $(, $arg)*> IntoFn<'gc, ($($arg,)*)> for F
        where
            F: Fn(Ctx<'gc> $(, $arg)*) -> R + Copy + 'static,
            $($arg: MimasType<'gc>,)*
            R: IntoNativeResult<'gc>,
        {
            #[allow(non_snake_case, unused_variables, unused_mut)]
            fn install(self, api: &mut Api<'_, 'gc>, name: String, module: Vec<String>) -> NativeId {
                let reg = api.library.registry();
                let parameters = vec![$(<$arg as MimasType<'gc>>::mimas_ty(reg),)*];
                let return_ty = <R as IntoNativeResult<'gc>>::return_ty(reg);
                let doc = doc_for(std::any::type_name_of_val(&self));
                let native = make_native(&api.ctx, move |ctx, args| {
                    let mut it = args.iter().copied();
                    $(let $arg = <$arg as MimasType<'gc>>::from_value(
                        ctx,
                        it.next().expect("native called with too few args"),
                    ).map_err(RtErr::from)?;)*
                    self(ctx $(, $arg)*).into_native_result(ctx)
                });
                let id = api.library.function(ApiFunction {
                    name, module, parameters, return_ty, doc, call: (),
                });
                api.store_native(id, native);
                id
            }

            #[allow(non_snake_case, unused_variables, unused_mut)]
            fn install_assoc(self, api: &mut Api<'_, 'gc>, recv_ty: Ty, name: String) {
                let reg = api.library.registry();
                let parameters = vec![$(<$arg as MimasType<'gc>>::mimas_ty(reg),)*];
                let return_ty = <R as IntoNativeResult<'gc>>::return_ty(reg);
                let doc = doc_for(std::any::type_name_of_val(&self));
                let native = make_native(&api.ctx, move |ctx, args| {
                    let mut it = args.iter().copied();
                    $(let $arg = <$arg as MimasType<'gc>>::from_value(
                        ctx,
                        it.next().expect("native called with too few args"),
                    ).map_err(RtErr::from)?;)*
                    self(ctx $(, $arg)*).into_native_result(ctx)
                });
                let id = api.library.method(ApiMethod {
                    recv_ty,
                    name,
                    parameters,
                    return_ty,
                    takes_self: false,
                    doc,
                    call: (),
                });
                api.store_native(id, native);
            }
        }
    };
}

// something something more tuples
impl_into_fn!();
impl_into_fn!(A);
impl_into_fn!(A, B);
impl_into_fn!(A, B, C);
impl_into_fn!(A, B, C, D);
impl_into_fn!(A, B, C, D, E);
impl_into_fn!(A, B, C, D, E, F1);
impl_into_fn!(A, B, C, D, E, F1, G);
impl_into_fn!(A, B, C, D, E, F1, G, H);

macro_rules! impl_into_method {
    ($($arg:ident),*) => {
        impl<'gc, F, Recv, R $(, $arg)*> IntoMethod<'gc, (Recv, $($arg,)*)> for F
        where
            F: Fn(Ctx<'gc>, Recv $(, $arg)*) -> R + Copy + 'static,
            Recv: MimasType<'gc>,
            $($arg: MimasType<'gc>,)*
            R: IntoNativeResult<'gc>,
        {
            #[allow(non_snake_case, unused_variables, unused_mut)]
            fn install_method(self, api: &mut Api<'_, 'gc>, name: String) -> NativeId {
                let reg = api.library.registry();
                let recv_ty = <Recv as MimasType<'gc>>::mimas_ty(reg)
                    .expect("native method receiver must have a concrete Ty");
                let parameters = vec![$(<$arg as MimasType<'gc>>::mimas_ty(reg),)*];
                let return_ty = <R as IntoNativeResult<'gc>>::return_ty(reg);
                let doc = doc_for(std::any::type_name_of_val(&self));
                let native = make_native(&api.ctx, move |ctx, args| {
                    let mut it = args.iter().copied();
                    let recv = <Recv as MimasType<'gc>>::from_value(
                        ctx,
                        it.next().expect("native called without receiver"),
                    ).map_err(RtErr::from)?;
                    $(let $arg = <$arg as MimasType<'gc>>::from_value(
                        ctx,
                        it.next().expect("native called with too few args"),
                    ).map_err(RtErr::from)?;)*
                    self(ctx, recv $(, $arg)*).into_native_result(ctx)
                });
                let id = api.library.method(ApiMethod {
                    recv_ty,
                    name,
                    parameters,
                    return_ty,
                    takes_self: true,
                    doc,
                    call: (),
                });
                api.store_native(id, native);
                id
            }
        }
    };
}

impl_into_method!();
impl_into_method!(A);
impl_into_method!(A, B);
impl_into_method!(A, B, C);
impl_into_method!(A, B, C, D);
impl_into_method!(A, B, C, D, E);
impl_into_method!(A, B, C, D, E, F1);
impl_into_method!(A, B, C, D, E, F1, G);

fn short_name_of_val<T: ?Sized>(value: &T) -> String {
    let full = std::any::type_name_of_val(value);
    full.rsplit("::").next().unwrap_or(full).to_string()
}

pub fn install_into<'gc, F>(ctx: Ctx<'gc>, install_fn: F) -> Library<()>
where
    F: FnOnce(&mut Api<'_, 'gc>),
{
    let mut library = Library::<()>::new();
    let mut api = Api {
        ctx,
        library: &mut library,
    };
    // register every `#[mimas]` item linked into the binary (from any crate), interleaved with the
    // caller's explicit install so dependencies resolve regardless of source:
    // 1. `#[mimas]` adts first -- a fn may name them.
    // 2. the caller's `install_fn` (e.g. library::std), which registers its own adts + fns in a
    //    self-consistent order.
    // 3. `#[mimas]` fns last -- so they can name both `#[mimas]` adts and anything `install_fn`
    //    registered (e.g. a std type).
    // (inventory iteration order is otherwise unspecified; the two phases come from MimasReg.)
    for reg in inventory::iter::<MimasReg> {
        if reg.phase == MimasReg::PHASE_ADT {
            (reg.register)(&mut api);
        }
    }
    install_fn(&mut api);
    for reg in inventory::iter::<MimasReg> {
        if reg.phase != MimasReg::PHASE_ADT {
            (reg.register)(&mut api);
        }
    }
    library
}
