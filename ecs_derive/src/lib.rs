use darling::{util::SpannedValue, FromDeriveInput};
use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(darling::FromDeriveInput)]
#[darling(attributes(ammo))]
struct MacroInput {
    int_namespace: SpannedValue<syn::LitStr>,
    int_id: SpannedValue<syn::LitStr>,
    namespace: SpannedValue<syn::LitStr>,
    id: SpannedValue<syn::LitStr>,

    ident: syn::Ident,
    generics: SpannedValue<syn::Generics>,
}

#[proc_macro_derive(Component, attributes(ammo))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let derive_input: DeriveInput = parse_macro_input!(input);
    let input = match MacroInput::from_derive_input(&derive_input) {
        Err(e) => return e.write_errors().into(),
        Ok(x) => x,
    };

    let MacroInput {
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
        impl<#generics> ammo_ecs_core::Component for #ident<#generics> {
            fn get_string_id() -> ammo_ecs_core::component::StringComponentId {
                ammo_ecs_core::StringComponentId { namespace: #namespace, id: #id }
            }

            fn get_int_id() -> ammo_ecs_core::component::IntComponentId {
                ammo_ecs_core::component::IntComponentId {
                    namespace: unsafe { std::num::NonZeroU16::new_unchecked(#int_namespace) },
                    id: #int_id,
                }
            }
        }
    };

    out.into()
}
