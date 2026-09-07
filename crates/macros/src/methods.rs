use crate::{collect_doc, convert::expand_conversion, doc_submission, register::submission};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    FnArg, Ident, ImplItem, ImplItemFn, ItemFn, ItemImpl, Pat, Type, parse_quote,
    visit_mut::VisitMut,
};

pub fn expand_impl(block: ItemImpl) -> Result<TokenStream2, syn::Error> {
    if let Some((_, path, _)) = &block.trait_ {
        return Err(syn::Error::new_spanned(
            path,
            "`#[mimas]` supports only inherent impls",
        ));
    }
    if !block.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &block.generics,
            "`#[mimas]` impls cannot be generic -- mimas has no generics",
        ));
    }
    let self_ident = match &*block.self_ty {
        Type::Path(tp) => tp.path.get_ident().cloned(),
        _ => None,
    }
    .ok_or_else(|| {
        syn::Error::new_spanned(&block.self_ty, "`#[mimas]` impls need a plain type name")
    })?;

    let mut out = TokenStream2::new();
    let mut adds = TokenStream2::new();
    for item in &block.items {
        match item {
            ImplItem::Fn(method) => {
                let (shim, add) = method_shim(&self_ident, method)?;
                let doc = doc_submission(&shim.sig.ident, &collect_doc(&method.attrs));
                out.extend(quote!(#shim #doc));
                adds.extend(add);
            }
            ImplItem::Const(c) => {
                let ident = &c.ident;
                let name = ident.to_string();
                let (ty, value) =
                    crate::register::const_literal(&c.ty, &quote!(#self_ident::#ident))?;
                let doc = collect_doc(&c.attrs);
                adds.extend(quote! {
                    let __recv = api.ty_of::<#self_ident>();
                    api.assoc_constant(__recv, #name, #ty, #value, #doc);
                });
            }
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "`#[mimas]` impls support only fns and consts",
                ));
            }
        }
    }
    let registration = submission(
        &format_ident!("impl_{}", self_ident),
        quote!(PHASE_FN),
        adds,
    );
    Ok(quote!(#block #out #registration))
}

/// One method's shim plus its `api.add_*` call. The shim's signature is the method's minus
/// `self` (with `Self` spelled concretely), run through the shared conversion, with the
/// receiver inserted after ctx; its body delegates via UFCS.
fn method_shim(
    self_ident: &Ident,
    method: &ImplItemFn,
) -> Result<(ItemFn, TokenStream2), syn::Error> {
    let name = &method.sig.ident;
    let name_str = name.to_string();
    if method
        .sig
        .generics
        .params
        .iter()
        .any(|p| !matches!(p, syn::GenericParam::Lifetime(_)))
    {
        return Err(syn::Error::new_spanned(
            &method.sig.generics,
            "`#[mimas]` methods cannot be generic -- mimas has no generics",
        ));
    }

    let receiver = match method.sig.inputs.first() {
        Some(FnArg::Receiver(r)) => {
            if r.colon_token.is_some() {
                return Err(syn::Error::new_spanned(
                    r,
                    "`#[mimas]` methods take `self`, `&self`, or `&mut self`",
                ));
            }
            Some(r.clone())
        }
        _ => None,
    };

    // delegation passes every non-self parameter by its name, in the user's order -- after
    // conversion those names are bound to the views the method's own signature expects
    let mut call_args: Vec<Ident> = Vec::new();
    for arg in &method.sig.inputs {
        if let FnArg::Typed(pt) = arg {
            let Pat::Ident(p) = &*pt.pat else {
                return Err(syn::Error::new_spanned(
                    &pt.pat,
                    "`#[mimas]` method parameters must be plain names",
                ));
            };
            call_args.push(p.ident.clone());
        }
    }

    let mut shim = ItemFn {
        attrs: vec![
            parse_quote!(#[doc(hidden)]),
            parse_quote!(#[allow(non_snake_case)]),
        ],
        vis: syn::Visibility::Inherited,
        sig: method.sig.clone(),
        block: parse_quote!({}),
    };
    shim.sig.ident = format_ident!("__mimas_{}_{}", self_ident, name);
    if receiver.is_some() {
        let inputs = shim.sig.inputs.into_iter().skip(1).collect();
        shim.sig.inputs = inputs;
    }
    ReplaceSelf(self_ident).visit_signature_mut(&mut shim.sig);

    let ctx = expand_conversion(&mut shim)?;
    let vm = crate::vm_path();
    let body: Vec<syn::Stmt> = match &receiver {
        None => parse_quote!(return #self_ident::#name( #(#call_args),* );),
        Some(r) if r.reference.is_some() && r.mutability.is_some() => {
            // load/store tie the recv and ctx to the same 'gc. a method that takes its own
            // ctx skips the inject path, so 'gc may be missing and the ctx param's lifetime
            // may be elided -- respell it in the shim (the method itself is untouched)
            crate::convert::ensure_gc(&mut shim.sig);
            if let Some(FnArg::Typed(pt)) = shim.sig.inputs.first_mut()
                && let Type::Path(tp) = &mut *pt.ty
                && let Some(seg) = tp.path.segments.last_mut()
                && seg.ident == "Ctx"
            {
                seg.arguments = syn::PathArguments::AngleBracketed(parse_quote!(<'gc>));
            }
            shim.sig.inputs.insert(
                1,
                parse_quote!(__recv: #vm::adt::InstanceOf<'gc, #self_ident>),
            );
            parse_quote! {
                let mut __this: #self_ident = __recv.load(#ctx);
                let __out = #self_ident::#name(&mut __this #(, #call_args)*);
                __recv.store(#ctx, __this);
                return __out;
            }
        }
        Some(r) => {
            shim.sig.inputs.insert(1, parse_quote!(__this: #self_ident));
            let recv = if r.reference.is_some() {
                quote!(&__this)
            } else {
                quote!(__this)
            };
            parse_quote!(return #self_ident::#name(#recv #(, #call_args)*);)
        }
    };
    shim.block.stmts.extend(body);

    let shim_ident = &shim.sig.ident;
    let add = if receiver.is_some() {
        quote!(api.add_method_named(#name_str, #shim_ident);)
    } else {
        quote!(api.add_assoc_of::<#self_ident, _, _>(#name_str, #shim_ident);)
    };
    Ok((shim, add))
}

struct ReplaceSelf<'a>(&'a Ident);

impl VisitMut for ReplaceSelf<'_> {
    fn visit_ident_mut(&mut self, i: &mut Ident) {
        if i == "Self" {
            *i = self.0.clone();
        }
    }
}
