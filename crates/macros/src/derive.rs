use crate::{collect_doc, vm_path};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Type};

pub fn expand_derive(input: &DeriveInput, is_enum: bool) -> Result<TokenStream2, syn::Error> {
    let (macro_name, noun) = if is_enum {
        ("MimasEnum", "enums")
    } else {
        ("MimasStruct", "structs")
    };
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            format!("{macro_name} cannot be derived on generic {noun}"),
        ));
    }

    let vm = vm_path();
    let name = &input.ident;
    let (kind, variants): (_, Vec<(TokenStream2, String, String, &Fields)>) = match &input.data {
        Data::Enum(data) if is_enum => (
            quote!(#vm::ApiAdtKind::Enum),
            data.variants
                .iter()
                .map(|v| {
                    let v_ident = &v.ident;
                    (
                        quote!(#name::#v_ident),
                        v_ident.to_string(),
                        collect_doc(&v.attrs),
                        &v.fields,
                    )
                })
                .collect(),
        ),
        Data::Struct(data) if !is_enum => (
            quote!(#vm::ApiAdtKind::Struct),
            vec![(quote!(#name), "@".to_string(), String::new(), &data.fields)],
        ),
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                format!("{macro_name} can only be derived on {noun}"),
            ));
        }
    };

    let mut descriptor_entries = Vec::new();
    let mut into_arms = Vec::new();
    let mut from_arms = Vec::new();
    for (disc, (ctor, name_str, doc, fields)) in variants.iter().enumerate() {
        let (d, i, f) = variant_tokens(ctor, name_str, doc, fields, disc, &vm);
        descriptor_entries.push(d);
        into_arms.push(i);
        from_arms.push(f);
    }

    let name_str = name.to_string();
    let adt_doc = collect_doc(&input.attrs);
    let missing_build = format!(
        "Type `{name_str}` not registered via `api.add_adt::<{name_str}>()` before being referenced in a mimas Ty.",
    );
    let missing_runtime = format!(
        "Type `{name_str}` has no runtime binding; `api.add_adt::<{name_str}>()` must run during install.",
    );

    Ok(quote! {
        impl #vm::adt::MimasAdt for #name {
            fn descriptor(reg: &#vm::Registry) -> #vm::adt::ApiAdtDescriptor {
                #vm::adt::ApiAdtDescriptor {
                    name: #name_str,
                    module: &[],
                    kind: #kind,
                    doc: #adt_doc,
                    variants: ::std::vec![ #(#descriptor_entries),* ],
                }
            }
        }

        impl<'gc> #vm::conversion::MimasType<'gc> for #name {
            fn mimas_ty(reg: &#vm::Registry) -> ::std::option::Option<#vm::Ty> {
                let binding = reg.get::<Self>().expect(#missing_build);
                ::std::option::Option::Some(#vm::Ty::Adt(binding.adt_id))
            }

            fn from_value(
                ctx: #vm::Ctx<'gc>,
                v: #vm::Val<'gc>,
            ) -> ::std::result::Result<Self, #vm::conversion::TypeError> {
                let inst = match v {
                    #vm::Val::Instance(i) => i,
                    other => return ::std::result::Result::Err(#vm::conversion::TypeError {
                        expected: #name_str.into(),
                        got: ::std::format!("{:?}", other),
                    }),
                };
                let (struct_id, fields) = {
                    let b = inst.0.borrow();
                    (b.struct_id, b.fields.clone())
                };
                let binding = ctx
                    .state()
                    .mimas_bindings
                    .borrow()
                    .0
                    .get(&::std::any::TypeId::of::<Self>())
                    .expect(#missing_runtime)
                    .clone();
                let disc = binding
                    .variant_layout_ids
                    .iter()
                    .position(|id| (id.index() as u32) == struct_id)
                    .ok_or_else(|| #vm::conversion::TypeError {
                        expected: #name_str.into(),
                        got: ::std::format!("instance@{}", struct_id),
                    })?;
                match disc {
                    #(#from_arms,)*
                    _ => ::std::result::Result::Err(#vm::conversion::TypeError {
                        expected: #name_str.into(),
                        got: ::std::format!("instance@{}", struct_id),
                    }),
                }
            }

            fn into_value(self, ctx: #vm::Ctx<'gc>) -> #vm::Val<'gc> {
                let binding = ctx
                    .state()
                    .mimas_bindings
                    .borrow()
                    .0
                    .get(&::std::any::TypeId::of::<Self>())
                    .expect(#missing_runtime)
                    .clone();
                let (disc, fields): (usize, ::std::vec::Vec<#vm::Val<'gc>>) = match self {
                    #(#into_arms,)*
                };
                let layout_id = binding.variant_layout_ids[disc];
                #vm::Val::Instance(ctx.new_instance(layout_id.index() as u32, #vm::Fields::new(fields)))
            }
        }
    })
}

/// One variant's (descriptor entry, `into_value` arm, `from_value` arm), all built from the
/// same normalized field list so unit / tuple / named handling lives in one place.
fn variant_tokens(
    ctor: &TokenStream2,
    name_str: &str,
    doc: &str,
    fields: &Fields,
    disc: usize,
    vm: &TokenStream2,
) -> (TokenStream2, TokenStream2, TokenStream2) {
    let (names, tys): (Vec<syn::Ident>, Vec<&Type>) = match fields {
        Fields::Unit => (vec![], vec![]),
        Fields::Unnamed(f) => (
            (0..f.unnamed.len())
                .map(|j| format_ident!("__f{j}"))
                .collect(),
            f.unnamed.iter().map(|f| &f.ty).collect(),
        ),
        Fields::Named(f) => (
            f.named.iter().map(|f| f.ident.clone().unwrap()).collect(),
            f.named.iter().map(|f| &f.ty).collect(),
        ),
    };
    let pattern = match fields {
        Fields::Unit => quote!(#ctor),
        Fields::Unnamed(_) => quote!(#ctor( #(#names),* )),
        Fields::Named(_) => quote!(#ctor { #(#names),* }),
    };

    let descriptor_fields = match fields {
        Fields::Unit => quote!(#vm::ApiVariantFields::Unit),
        Fields::Unnamed(_) => quote!(#vm::ApiVariantFields::Tuple(::std::vec![
            #(<#tys as #vm::conversion::MimasType<'static>>::mimas_ty(reg)
                .expect("field type has no concrete Ty")),*
        ])),
        Fields::Named(_) => {
            let field_names = names.iter().map(|n| n.to_string());
            quote!(#vm::ApiVariantFields::Named(::std::vec![ #((
                #field_names.to_string(),
                <#tys as #vm::conversion::MimasType<'static>>::mimas_ty(reg)
                    .expect("field type has no concrete Ty"),
            )),* ]))
        }
    };
    let descriptor = quote!(#vm::adt::ApiVariantShape {
        name: #name_str.to_string(),
        doc: #doc,
        fields: #descriptor_fields,
    });

    let into = quote!(#pattern => (
        #disc,
        ::std::vec![ #(<#tys as #vm::conversion::MimasType<'gc>>::into_value(#names, ctx)),* ],
    ));

    let extracts: Vec<TokenStream2> = tys
        .iter()
        .map(|t| {
            quote!(<#t as #vm::conversion::MimasType<'gc>>::from_value(
                ctx,
                it.next().ok_or_else(|| #vm::conversion::TypeError {
                    expected: "field".into(),
                    got: "<empty>".into(),
                })?,
            )?)
        })
        .collect();
    let construct = match fields {
        Fields::Unit => quote!(#ctor),
        Fields::Unnamed(_) => quote!(#ctor( #(#extracts),* )),
        Fields::Named(_) => quote!(#ctor { #(#names: #extracts),* }),
    };
    let from = if tys.is_empty() {
        quote!(#disc => ::std::result::Result::Ok(#construct))
    } else {
        quote!(#disc => {
            let mut it = fields.into_iter();
            ::std::result::Result::Ok(#construct)
        })
    };

    (descriptor, into, from)
}
