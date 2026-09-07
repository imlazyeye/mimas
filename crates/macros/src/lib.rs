//! Proc-macros for authoring mimas natives from Rust: two attribute macros that split along
//! the conversion-vs-registration line, plus the `MimasEnum` / `MimasStruct` derives.
//!
//! Every native fn the VM calls has to be written in a particular shape (a `Ctx<'gc>` first
//! parameter, gc handles instead of borrowed slices -- see [`convert`]), and *separately* has
//! to be registered into a library so mimas code can find it:
//!
//! - [`macro@native`] does **conversion only**. You register the result yourself with an explicit
//!   `api.add_method(..)` / `api.add_assoc(..)` / `api.add(..)` call. This is what the standard
//!   library's builtin methods (e.g. `array.push`, `int.random`) use, because they attach to a
//!   *receiver `Ty`* the macro can't infer from tokens alone.
//!
//! - [`macro@mimas`] does **registration** (plus, for fns, the same conversion). Accepts a free fn,
//!   a struct / enum (emits the derive impls too -- don't also `#[derive]`), a const of a primitive
//!   type, or an inherent impl block (see [`methods`]). Bare `#[mimas]` targets the prelude,
//!   `#[mimas(foo::bar)]` a module path. Registration is deferred through the [`inventory`] crate
//!   -- see [`register`] and `vm::api::MimasReg`.

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, Ident, parse_macro_input};

mod convert;
mod derive;
mod methods;
mod register;

/// Conversion only -- rewrites a `fn(Ctx<'gc>, ..)` into the shape the VM's `IntoFn` /
/// `IntoMethod` machinery accepts; you register it yourself. For a free fn that should just
/// land in the prelude, use [`macro@mimas`] instead and skip the manual `api.add`.
#[proc_macro_attribute]
pub fn native(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as syn::ItemFn);
    let submission = doc_submission(&input.sig.ident, &collect_doc(&input.attrs));
    if let Err(e) = convert::expand_conversion(&mut input) {
        return e.to_compile_error().into();
    }
    TokenStream::from(quote!(#input #submission))
}

/// Conversion **and** automatic registration -- no manual `api.add*` call.
///
/// ```ignore
/// #[mimas]                       // -> registered into the prelude
/// fn greet(ctx: Ctx<'_>, who: &str) -> String { format!("hi {who}") }
///
/// #[mimas(std::fs)]              // -> registered into the `std::fs` module
/// fn read(path: &str) -> Raisable<String> { /* .. */ }
///
/// #[mimas]                       // -> methods/assoc fns on a #[mimas] adt
/// impl Player { fn damage(&mut self, amount: i64) { /* .. */ } }
/// ```
///
/// There's no `Api` in scope at the definition site, so the call is deferred: the macro emits a
/// wrapper fn holding it plus an `inventory::submit!` -- see [`register`] and
/// `vm::api::MimasReg` for how wrappers are collected at install time.
#[proc_macro_attribute]
pub fn mimas(attr: TokenStream, item: TokenStream) -> TokenStream {
    match expand_mimas(attr, item) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_derive(MimasEnum)]
pub fn derive_mimas_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive::expand_derive(&input, true) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_derive(MimasStruct)]
pub fn derive_mimas_struct(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive::expand_derive(&input, false) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_mimas(attr: TokenStream, item: TokenStream) -> Result<TokenStream2, syn::Error> {
    // `#[mimas]` registers into the prelude; `#[mimas(foo::bar)]` names a module path
    let module: Option<String> = if attr.is_empty() {
        None
    } else {
        let path: syn::Path = syn::parse(attr)?;
        Some(
            path.segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
        )
    };

    match syn::parse::<syn::Item>(item)? {
        syn::Item::Fn(mut function) => {
            let submission = doc_submission(&function.sig.ident, &collect_doc(&function.attrs));
            convert::expand_conversion(&mut function)?;
            let registration = register::fn_registration(&function.sig.ident, module.as_deref());
            Ok(quote!(#function #registration #submission))
        }
        // struct / enum: emit the same impls the derives would (so don't *also* `#[derive]`
        // them) plus the `add_adt` submission
        syn::Item::Struct(item) => {
            let di = DeriveInput::from(item);
            let impls = derive::expand_derive(&di, false)?;
            let registration = register::adt_registration(&di.ident, module.as_deref());
            Ok(quote!(#di #impls #registration))
        }
        syn::Item::Enum(item) => {
            let di = DeriveInput::from(item);
            let impls = derive::expand_derive(&di, true)?;
            let registration = register::adt_registration(&di.ident, module.as_deref());
            Ok(quote!(#di #impls #registration))
        }
        syn::Item::Const(item) => {
            let registration = register::const_registration(&item, module.as_deref())?;
            Ok(quote!(#item #registration))
        }
        syn::Item::Impl(block) => {
            if module.is_some() {
                return Err(syn::Error::new_spanned(
                    &block.self_ty,
                    "methods attach to their adt, not a module -- drop the path argument",
                ));
            }
            methods::expand_impl(block)
        }
        other => Err(syn::Error::new_spanned(
            other,
            "`#[mimas]` supports free fns, structs, enums, consts, and inherent impl blocks.",
        )),
    }
}

// proc_macro_crate v3 ignores [lib.name] in integration tests -- it returns the sanitized
// package name (`mimas_vm`) but the only importable name is the lib name (`vm`).
fn vm_path() -> TokenStream2 {
    if let Ok(found) = crate_name("mimas-vm") {
        let name = match found {
            FoundCrate::Itself => "vm",
            FoundCrate::Name(n) if n == "mimas_vm" => "vm",
            FoundCrate::Name(n) => {
                let ident = Ident::new(&n, proc_macro2::Span::call_site());
                return quote!(::#ident);
            }
        };
        let ident = Ident::new(name, proc_macro2::Span::call_site());
        return quote!(::#ident);
    }
    if let Ok(FoundCrate::Name(name)) = crate_name("mimas") {
        let ident = Ident::new(&name, proc_macro2::Span::call_site());
        return quote!(::#ident::vm);
    }
    quote!(::vm)
}

/// Collects an item's leading `///` doc-comment (each line stripped of the one leading space
/// rustdoc adds). `#[doc = "..."]` is how the compiler desugars `///`, so both spellings work.
fn collect_doc(attrs: &[syn::Attribute]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        let syn::Meta::NameValue(nv) = &attr.meta else {
            continue;
        };
        let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) = &nv.value
        else {
            continue;
        };
        lines.push(
            s.value()
                .strip_prefix(' ')
                .map(str::to_string)
                .unwrap_or_else(|| s.value()),
        );
    }
    lines.join("\n")
}

/// Ships a doc-comment to install time keyed by the item's full Rust path, so `vm::api::doc_for`
/// can join it onto the registered `ApiFunction`/`ApiMethod` (whose receiver/module are known
/// only at the `api.add_*` call site, not here).
fn doc_submission(fn_ident: &Ident, doc: &str) -> TokenStream2 {
    if doc.is_empty() {
        return TokenStream2::new();
    }
    let vm = vm_path();
    let name = fn_ident.to_string();
    quote! {
        #vm::inventory::submit! {
            #vm::api::NativeDoc {
                path: ::std::concat!(::std::module_path!(), "::", #name),
                doc: #doc,
            }
        }
    }
}
