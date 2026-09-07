use crate::vm_path;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    FnArg, GenericArgument, Ident, ItemFn, Pat, PathArguments, PathSegment, Type, parse_quote,
    spanned::Spanned,
};

/// Returns the name of the ctx binding so callers can reference it in generated code.
pub fn expand_conversion(input: &mut ItemFn) -> Result<Ident, syn::Error> {
    let ctx = prepare_ctx(input)?;
    let mut preludes: Vec<syn::Stmt> = Vec::new();
    for arg in input.sig.inputs.iter_mut().skip(1) {
        let FnArg::Typed(pt) = arg else { continue };
        let Pat::Ident(pi) = &*pt.pat else { continue };
        let name = pi.ident.clone();
        let Some(shape) = detect_shape(&pt.ty)? else {
            continue;
        };
        // the rebinding is annotated with the user's written type -- that's what keeps their
        // `use HashMap` / `use Str` imports live after the swap
        let original_ty = (*pt.ty).clone();
        *pt.ty = target_handle(&shape);
        preludes.extend(prelude_for(&name, &shape, &ctx, &original_ty));
    }
    for stmt in preludes.into_iter().rev() {
        input.block.stmts.insert(0, stmt);
    }
    Ok(ctx)
}

fn prepare_ctx(input: &mut ItemFn) -> Result<Ident, syn::Error> {
    if let Some(FnArg::Typed(pt)) = input.sig.inputs.first()
        && let Type::Path(tp) = &*pt.ty
        && tp.path.segments.last().is_some_and(|s| s.ident == "Ctx")
    {
        let Pat::Ident(i) = &*pt.pat else {
            return Err(syn::Error::new_spanned(
                &pt.pat,
                "the `Ctx` parameter must be named -- generated code may reference it",
            ));
        };
        return Ok(i.ident.clone());
    }
    ensure_gc(&mut input.sig);
    let vm = vm_path();
    input
        .sig
        .inputs
        .insert(0, parse_quote!(__ctx: #vm::Ctx<'gc>));
    Ok(Ident::new("__ctx", proc_macro2::Span::call_site()))
}

pub fn ensure_gc(sig: &mut syn::Signature) {
    let has_gc = sig
        .generics
        .params
        .iter()
        .any(|p| matches!(p, syn::GenericParam::Lifetime(l) if l.lifetime.ident == "gc"));
    if !has_gc {
        sig.generics.params.insert(0, parse_quote!('gc));
    }
}

/// What a borrowed container parameter expands to: every recognized form is a point in the
/// `dict` x `mutable` x [`Mode`] matrix.
pub struct Shape {
    dict: bool,
    mutable: bool,
    mode: Mode,
}

enum Mode {
    /// `Val` elements -- borrow the gc storage directly, no copy.
    Borrow,
    /// `anon::T` elements -- `Anon<N>` is a transparent `Val`, so this also borrows directly
    /// and just reinterprets. The slot tokens ride into the registered type via
    /// `ArrayOf<'gc, N>` / `DictOf<'gc, N>` (the bare handles would erase N to 0).
    BorrowAnon(TokenStream2),
    /// Real typed elements, read-only -- `IntoFn` already materialized the concrete `Vec<T>` /
    /// `HashMap<K, V>` via `MimasType::from_value`, so the prelude just rebinds the user's
    /// borrowed view. Registered with the precise shape (`Array(Int)`, not `Array(Anon(0))`),
    /// which is what receiver-overload dispatch unifies against.
    Materialize(Vec<Type>),
}

fn detect_shape(ty: &Type) -> Result<Option<Shape>, syn::Error> {
    let Type::Reference(r) = ty else {
        return Ok(None);
    };
    let mutable = r.mutability.is_some();
    let inner = &*r.elem;

    // &[T] / &Vec<T>
    let array_elem = match inner {
        Type::Slice(s) => Some((*s.elem).clone()),
        _ => path_segment(inner, "Vec").and_then(|seg| generic_types(seg).into_iter().next()),
    };
    if let Some(elem) = array_elem {
        let mode = if is_val(&elem) {
            Mode::Borrow
        } else if let Some(slot) = anon_slot(&elem) {
            Mode::BorrowAnon(slot)
        } else if mutable {
            return Err(reject_mut_typed(inner.span()));
        } else {
            Mode::Materialize(vec![elem])
        };
        return Ok(Some(Shape {
            dict: false,
            mutable,
            mode,
        }));
    }

    // &DictMap<'gc[, V]> -- the value arg picks the view: absent/`Val` = permissive,
    // `anon::T` = typed reinterpret
    if let Some(seg) = path_segment(inner, "DictMap") {
        let mode = match generic_types(seg).into_iter().next() {
            None => Mode::Borrow,
            Some(t) if is_val(&t) => Mode::Borrow,
            Some(t) => match anon_slot(&t) {
                Some(slot) => Mode::BorrowAnon(slot),
                None => {
                    return Err(syn::Error::new(
                        t.span(),
                        "DictMap value type must be `Val<'gc>` or `anon::T<'gc>` -- for typed values take a read-only `&HashMap<K, V>` instead.",
                    ));
                }
            },
        };
        return Ok(Some(Shape {
            dict: true,
            mutable,
            mode,
        }));
    }

    // &HashMap<K, V> -- typed dict, read-only
    if let Some(seg) = path_segment(inner, "HashMap")
        && let [k, v] = &generic_types(seg)[..]
    {
        if mutable {
            return Err(reject_mut_typed(inner.span()));
        }
        return Ok(Some(Shape {
            dict: true,
            mutable,
            mode: Mode::Materialize(vec![k.clone(), v.clone()]),
        }));
    }

    Ok(None)
}

fn target_handle(shape: &Shape) -> Type {
    let vm = vm_path();
    match (&shape.mode, shape.dict) {
        (Mode::Borrow, false) => parse_quote!(#vm::Array<'gc>),
        (Mode::Borrow, true) => parse_quote!(#vm::Dict<'gc>),
        (Mode::BorrowAnon(slot), false) => parse_quote!(#vm::anon::ArrayOf<'gc, { #slot }>),
        (Mode::BorrowAnon(slot), true) => parse_quote!(#vm::anon::DictOf<'gc, { #slot }>),
        (Mode::Materialize(t), false) => {
            let elem = &t[0];
            parse_quote!(::std::vec::Vec<#elem>)
        }
        (Mode::Materialize(t), true) => {
            let (k, v) = (&t[0], &t[1]);
            parse_quote!(::std::collections::HashMap<#k, #v>)
        }
    }
}

fn prelude_for(name: &Ident, shape: &Shape, ctx: &Ident, original_ty: &Type) -> Vec<syn::Stmt> {
    let guard = format_ident!("__{}_guard", name);
    let mut_kw = shape.mutable.then(|| quote!(mut));
    let borrow = if shape.mutable {
        quote!(borrow_mut(&#ctx))
    } else {
        quote!(borrow())
    };
    let deref = if shape.mutable {
        quote!(&mut *#guard)
    } else {
        quote!(&*#guard)
    };
    match &shape.mode {
        Mode::Borrow => parse_quote! {
            let #mut_kw #guard = #name.0.#borrow;
            let #name: #original_ty = #deref;
        },
        // the extra `.0` steps through the `ArrayOf`/`DictOf` wrapper to the gc handle
        Mode::BorrowAnon(_) => {
            let vm = vm_path();
            let cast = format_ident!(
                "as_{}_{}",
                if shape.dict { "dict" } else { "vec" },
                if shape.mutable { "mut" } else { "ref" }
            );
            parse_quote! {
                let #mut_kw #guard = #name.0.0.#borrow;
                let #name: #original_ty = #vm::anon::#cast(#deref);
            }
        }
        // `let _ = &ctx` keeps a user-written ctx param "used": the older prelude referenced
        // ctx for per-element conversion, so flipping a fn between materialize and borrow
        // shapes must not surface a new warning
        Mode::Materialize(_) => {
            let copy = format_ident!("__{}_copy", name);
            parse_quote! {
                let _ = &#ctx;
                let #copy = #name;
                let #name: #original_ty = &#copy;
            }
        }
    }
}

fn reject_mut_typed(span: proc_macro2::Span) -> syn::Error {
    syn::Error::new(
        span,
        "`&mut` typed-element collections aren't supported by #[mimas] -- mimas's gc storage holds `Val<'gc>`, not arbitrary types, so a mutable typed view would require a hidden copy-in / copy-out per call. Use `&mut Vec<Val<'gc>>` (or `&mut DictMap<'gc>`) and convert elements yourself, take a read-only `&[T]` / `&HashMap<K, V>` and return a new collection, or -- for a type-safe *generic* element -- use `&mut Vec<anon::T<'gc>>` / `&mut DictMap<'gc, anon::T<'gc>>`.",
    )
}

fn path_segment<'a>(ty: &'a Type, name: &str) -> Option<&'a PathSegment> {
    let Type::Path(tp) = ty else { return None };
    tp.path.segments.last().filter(|s| s.ident == name)
}

fn generic_types(seg: &PathSegment) -> Vec<Type> {
    let PathArguments::AngleBracketed(args) = &seg.arguments else {
        return vec![];
    };
    args.args
        .iter()
        .filter_map(|a| match a {
            GenericArgument::Type(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

fn is_val(ty: &Type) -> bool {
    matches!(ty, Type::Path(tp) if tp.path.segments.last().is_some_and(|s| s.ident == "Val"))
}

/// `anon::T` / `vm::anon::U` / `Anon<'gc, N>` -- a placeholder element type. Returns the slot's
/// tokens: the alias table for `T`/`U`/`V`/`W`, or the const argument lifted verbatim from an
/// explicit `Anon<'gc, N>` (never evaluated here -- rustc resolves consts/expressions).
fn anon_slot(ty: &Type) -> Option<TokenStream2> {
    let Type::Path(tp) = ty else {
        return None;
    };
    let seg = tp.path.segments.last()?;
    let in_anon = tp.path.segments.iter().any(|s| s.ident == "anon");
    match seg.ident.to_string().as_str() {
        "T" if in_anon => Some(quote!(0)),
        "U" if in_anon => Some(quote!(1)),
        "V" if in_anon => Some(quote!(2)),
        "W" if in_anon => Some(quote!(3)),
        "Anon" => {
            let PathArguments::AngleBracketed(args) = &seg.arguments else {
                return None;
            };
            // a literal parses as Const, a named const as Type; lifetimes are skipped
            args.args.iter().find_map(|a| match a {
                GenericArgument::Const(e) => Some(quote!(#e)),
                GenericArgument::Type(t) => Some(quote!(#t)),
                _ => None,
            })
        }
        _ => None,
    }
}
