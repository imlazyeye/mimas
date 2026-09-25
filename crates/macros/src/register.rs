use crate::vm_path;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Ident, Type};

/// `phase` is `PHASE_ADT` or `PHASE_FN` -- adts must install before any fn that names their
/// `Ty`, and inventory's iteration order is otherwise unspecified. The wrapper is generic over
/// `'a, 'gc` so the fn *item* coerces to the higher-ranked fn *pointer* in `MimasReg::register`.
pub fn submission(unique: &Ident, phase: TokenStream2, add: TokenStream2) -> TokenStream2 {
    let register_ident = format_ident!("__mimas_register_{}", unique);
    let vm = vm_path();
    quote! {
        #[doc(hidden)]
        #[allow(non_snake_case, clippy::unnecessary_cast)]
        fn #register_ident<'a, 'gc>(api: &mut #vm::api::Api<'a, 'gc>) {
            #add
        }
        #vm::inventory::submit! {
            #vm::api::MimasReg {
                phase: #vm::api::MimasReg::#phase,
                register: #register_ident,
            }
        }
    }
}

pub fn fn_registration(fn_ident: &Ident, module: Option<&str>) -> TokenStream2 {
    let add = match module {
        Some(path) => quote!(api.module(#path).add(#fn_ident);),
        None => quote!(api.add(#fn_ident);),
    };
    submission(fn_ident, quote!(PHASE_FN), add)
}

pub fn adt_registration(type_ident: &Ident, module: Option<&str>) -> TokenStream2 {
    let add = match module {
        Some(path) => quote!(api.module(#path).add_adt::<#type_ident>();),
        None => quote!(api.add_adt::<#type_ident>();),
    };
    submission(&format_ident!("adt_{}", type_ident), quote!(PHASE_ADT), add)
}

/// The mimas `Ty` and `Literal` come from the constant's Rust type via [`const_literal`]; the
/// wrapper reads the constant's actual value at install time (the const is re-emitted in the
/// same module, so the wrapper can name it).
pub fn const_registration(
    item: &syn::ItemConst,
    module: Option<&str>,
) -> Result<TokenStream2, syn::Error> {
    let ident = &item.ident;
    let name = ident.to_string();
    let (ty, value) = const_literal(&item.ty, &quote!(#ident))?;
    let doc = crate::collect_doc(&item.attrs);
    let add = match module {
        Some(path) => quote!(api.module(#path).constant(#name, #ty, #value, #doc);),
        None => quote!(api.constant(#name, #ty, #value, #doc);),
    };
    Ok(submission(
        &format_ident!("const_{}", ident),
        quote!(PHASE_FN),
        add,
    ))
}

/// Maps a constant's Rust type to its mimas `Ty` token and a `Literal`-construction expression
/// (which reads the constant at `path` -- a bare ident for free consts, `Type::NAME` for
/// associated ones).
pub fn const_literal(
    ty: &Type,
    path: &TokenStream2,
) -> Result<(TokenStream2, TokenStream2), syn::Error> {
    let vm = vm_path();
    let error = || {
        syn::Error::new_spanned(
            ty,
            "`#[mimas]` constants must be a primitive mimas type: bool, an integer, a float, or `&str`.",
        )
    };
    // &str is a reference, not a path -- handle it before the path cases
    if let Type::Reference(r) = ty
        && matches!(&*r.elem, Type::Path(tp) if tp.path.is_ident("str"))
    {
        return Ok((
            quote!(#vm::Ty::Str),
            quote!(#vm::Literal::Str(#path.to_string())),
        ));
    }
    let Type::Path(tp) = ty else {
        return Err(error());
    };
    let Some(seg) = tp.path.segments.last() else {
        return Err(error());
    };
    Ok(match seg.ident.to_string().as_str() {
        "f32" | "f64" => (
            quote!(#vm::Ty::Float),
            quote!(#vm::Literal::Float(#path as f64)),
        ),
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => (
            quote!(#vm::Ty::Int),
            quote!(#vm::Literal::Int(#path as i64)),
        ),
        "bool" => (quote!(#vm::Ty::Bool), quote!(#vm::Literal::Bool(#path))),
        "String" => (
            quote!(#vm::Ty::Str),
            quote!(#vm::Literal::Str(#path.to_string())),
        ),
        _ => return Err(error()),
    })
}
