use darling::{util::SpannedValue, FromDeriveInput};
use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(darling::FromDeriveInput)]
#[darling(attributes(ammo_component))]
struct ComponentMacroInput {
    ident: syn::Ident,
    generics: SpannedValue<syn::Generics>,
}

#[derive(darling::FromDeriveInput)]
#[darling(attributes(ammo_idents))]
struct HasIdentifiersMacroInput {
    int_namespace: SpannedValue<syn::LitStr>,
    int_id: SpannedValue<syn::LitStr>,
    namespace: SpannedValue<syn::LitStr>,
    id: SpannedValue<syn::LitStr>,

    ident: syn::Ident,
    generics: SpannedValue<syn::Generics>,
}

#[proc_macro_derive(Component, attributes(ammo_component))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let derive_input: DeriveInput = parse_macro_input!(input);
    let input = match ComponentMacroInput::from_derive_input(&derive_input) {
        Err(e) => return e.write_errors().into(),
        Ok(x) => x,
    };

    let ComponentMacroInput {
        generics, ident, ..
    } = input;

    let generics = &*generics;

    let out = quote! {
        impl<#generics> ammo_ecs_core::Component for #ident<#generics> {}
    };

    out.into()
}

#[proc_macro_derive(HasIdentifiers, attributes(ammo_idents))]
pub fn derive_has_identifiers(input: TokenStream) -> TokenStream {
    let derive_input: DeriveInput = parse_macro_input!(input);
    let input = match HasIdentifiersMacroInput::from_derive_input(&derive_input) {
        Err(e) => return e.write_errors().into(),
        Ok(x) => x,
    };

    let HasIdentifiersMacroInput {
        id,
        namespace,
        int_id,
        int_namespace,
        generics,
        ident,
    } = input;

    let id = id
        .parse::<syn::Expr>()
        .expect("id must b a valid expression");
    let namespace = namespace
        .parse::<syn::Expr>()
        .expect("namespace must be a valid expression");
    let int_id = int_id
        .parse::<syn::Expr>()
        .expect("int_id must be a valid expression");
    let int_namespace = int_namespace
        .parse::<syn::Expr>()
        .expect("int_namespace must be a valid expression");

    let generics = &*generics;

    let out = quote! {
        impl<#generics> ammo_ecs_core::HasIdentifiers for #ident<#generics> {
            fn get_string_id_from_type() -> ammo_ecs_core::StringId {
                ammo_ecs_core::StringId { namespace: #namespace, id: #id }
            }

            fn get_int_id_from_type() -> ammo_ecs_core::IntId {
                ammo_ecs_core::IntId {
                    namespace: unsafe { std::num::NonZeroU16::new_unchecked(#int_namespace) },
                    id: #int_id,
                }
            }
        }
    };

    out.into()
}
