extern crate proc_macro;
use proc_macro::TokenStream;

use quote::quote;
use quote::ToTokens;
use syn::{parse2, Generics, Type, TypeParamBound};

use proc_macro2::Ident;
use syn::{
    parse_macro_input, ConstParam, DeriveInput, GenericParam, ItemImpl, Lifetime, LifetimeDef,
    TypeParam,
};

// struct RenameLifetimeVisitor;
// impl VisitMut for RenameLifetimeVisitor {
//     // change all lifetimes to 'static
//     fn visit_lifetime_mut(&mut self, i: &mut Lifetime) {
//         let span = i.ident.span();
//         mem::replace(&mut i.ident, Ident::new("static", span));
//     }
//
//     // remove ?Sized bound
//     fn visit_predicate_type_mut(&mut self, i: &mut PredicateType) {
//         visit_predicate_type_mut(self, i);
//         let mut new_pred = i.clone();
//         let new_bounds = i
//             .bounds
//             .iter()
//             .filter(|&it| {
//                 if let TypeParamBound::Trait(TraitBound {
//                     modifier: TraitBoundModifier::Maybe(_),
//                     ..
//                 }) = it
//                 {
//                     false
//                 } else {
//                     true
//                 }
//             })
//             .cloned()
//             .collect();
//         mem::replace(&mut new_pred.bounds, new_bounds);
//     }
// }

// fn is_sized(bound: &TypeParamBound) -> bool {
//     if let TypeParamBound::Trait(TraitBound {
//         modifier: TraitBoundModifier::Maybe(_),
//         ..
//     }) = bound
//     {
//         false
//     } else {
//         true
//     }
// }

fn is_static(bound: &TypeParamBound) -> bool {
    if let TypeParamBound::Lifetime(Lifetime { ident, .. }) = bound {
        ident.to_string().eq("static")
    } else {
        false
    }
}

#[proc_macro_derive(Tid)]
pub fn my_derive(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident, generics, ..
    } = parse_macro_input!(input as DeriveInput);

    let type_params = generics
        .params
        .iter()
        .map(|it| match it {
            GenericParam::Type(TypeParam { ident, .. }) => quote! {#ident},
            GenericParam::Lifetime(LifetimeDef { lifetime, .. }) => quote! {#lifetime},
            GenericParam::Const(ConstParam { ident, .. }) => quote! {#ident},
        })
        .collect::<Vec<_>>();
    let type_ = if generics.lt_token.is_none() {
        quote! { #ident }
    } else {
        quote! { #ident<#(#type_params),*> }
    };
    let type_ = parse2(type_).unwrap();
    create_impl(generics, Box::new(type_), None).into()
}

fn create_impl(
    generics: Generics,
    type_: Box<Type>,
    hlq: Option<Ident>,
) -> proc_macro2::TokenStream {
    let hlq = hlq.map(|it| quote!(#it::)).unwrap_or(quote!());

    // no generics
    if generics.lt_token.is_none() {
        let tokens = quote! {
            unsafe impl<'a> #hlq ::better_any::TidAble<'a> for #type_{
                type Static = #type_;
            }
        };

        return tokens.into();
    }

    let lifetime_count = generics.lifetimes().count();
    if lifetime_count > 1 {
        unimplemented!("currently only single lifetime is supported")
    }
    let lifetime = generics
        .lifetimes()
        .next()
        .map(|it| it.lifetime.clone())
        .unwrap_or_else(|| syn::parse2(quote! {'a}).unwrap());
    let type_param_names = generics
        .type_params()
        .map(|it| &it.ident)
        .collect::<Vec<_>>();
    // let type_param_names2 = generics.type_params().map(|it| &it.ident);
    let const_param_names = generics
        .const_params()
        .map(|it| &it.ident)
        .collect::<Vec<_>>();
    // let const_param_names2 = generics.const_params().map(|it| &it.ident);

    // let where_clause = generics.where_clause.as_ref();
    let generic_params = &generics.params;
    let mut substitute_types = Vec::new();
    let mut generics_with_bounds = generics.clone();
    {
        let where_with_bounds = generics_with_bounds.make_where_clause();
        for generic in generic_params.iter() {
            if let GenericParam::Type(TypeParam { ident, bounds, .. }) = generic {
                // add Tid bound
                if bounds.iter().any(|it| is_static(it)) {
                    substitute_types.push(ident.to_token_stream())
                } else {
                    substitute_types.push(quote! {#ident::Static});
                    where_with_bounds.predicates.push(
                        syn::parse2(quote! {#ident: #hlq ::better_any::TidAble<#lifetime>})
                            .unwrap(),
                    );
                }
            }
        }
    }
    // remove defaults
    generics_with_bounds.params.iter_mut().for_each(|param| {
        if let GenericParam::Type(TypeParam { default, .. }) = param {
            *default = None
        }
    });
    let where_with_bounds = generics_with_bounds.where_clause.as_ref();
    let type_params_wo_defaults = &generics_with_bounds.params;

    let name = type_
        .to_token_stream()
        .to_string()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>();
    let temp_struct_ident = quote::format_ident!("__{}_should_never_exist", name);
    let tokens = if lifetime_count == 1 {
        quote! {
            unsafe impl<#type_params_wo_defaults> #hlq ::better_any::TidAble<#lifetime> for #type_
            #where_with_bounds {
                type Static = #temp_struct_ident<#(#substitute_types,)* #(#const_param_names,)*>;
            }
        }
    } else {
        // lifetime_count == 0
        quote! {
            unsafe impl<#lifetime,#type_params_wo_defaults> #hlq ::better_any::TidAble<#lifetime> for #type_
            #where_with_bounds {
                type Static = #temp_struct_ident<#(#substitute_types,)* #(#const_param_names,)*>;
            }
        }
    };

    // need to use separate struct becaus if we use original struct,
    // we have to forward all bounds
    let tokens = quote! {
        #tokens
        #[allow(warnings)]
        #[doc(hidden)]
        pub struct #temp_struct_ident<#(#type_param_names:?Sized,)* #(#const_param_names,)*>
            (#(core::marker::PhantomData<#type_param_names>,)* #(#const_param_names,)*);
    };

    return tokens;
}

#[proc_macro_attribute]
pub fn impl_tid(_params: TokenStream, input: TokenStream) -> TokenStream {
    if let ItemImpl {
        attrs,
        defaultness: None,
        unsafety: None,
        generics,
        trait_: Some((_, path, _)),
        self_ty,
        ..
    } = parse_macro_input!(input as ItemImpl)
    {
        let trait_ = path.segments.last().unwrap().ident.to_string();
        let hlq = path.segments.iter().nth_back(1).map(|it| it.ident.clone());
        if trait_ != "Tid" && trait_ != "TidAble" {
            panic!("supported only on implementations of Tid trait")
        }
        let impl_ = create_impl(generics, self_ty, hlq);
        return quote! {
            #(#attrs
            )*
            #impl_
        }
        .into();
    }
    panic!()
}
