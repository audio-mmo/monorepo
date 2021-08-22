use darling::{util::SpannedValue, FromDeriveInput};
use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(darling::FromDeriveInput)]
#[darling(attributes(ammo))]
struct MacroInput {
    int_namespace: SpannedValue<u16>,
    int_id: SpannedValue<u16>,
    namespace: SpannedValue<String>,
    id: SpannedValue<String>,

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

    let mut errors: Vec<darling::Error> = Default::default();
    if namespace.is_empty() {
        let e = darling::Error::custom("Namespace strings must not be empty").with_span(&namespace);
        errors.push(e);
    }
    if id.is_empty() {
        let e = darling::Error::custom("id may not be empty").with_span(&id);
        errors.push(e);
    }
    if *int_namespace == 0 {
        let e = darling::Error::custom("int_namespace may not be 0").with_span(&int_namespace);
        errors.push(e);
    }
    if !errors.is_empty() {
        return darling::Error::multiple(errors).write_errors().into();
    }

    let generics = &*generics;
    let namespace = &*namespace;
    let id = &*id;
    let int_id = &*int_id;
    let int_namespace = &*int_namespace;

    let out = quote! {
        impl<#generics> ammo_ecs::component::Component for #ident<#generics> {
            fn get_string_id() -> ammo_ecs::component::StringComponentId {
                ammo_ecs::component::StringComponentId { namespace: #namespace, id: #id }
            }

            fn get_int_id() -> ammo_ecs::component::IntComponentId {
                ammo_ecs::component::IntComponentId {
                    namespace: unsafe { std::num::NonZeroU16::new_unchecked(#int_namespace) },
                    id: #int_id,
                }
            }
        }
    };

    out.into()
}
