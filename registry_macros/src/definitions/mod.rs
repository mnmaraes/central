use quote::{quote, ToTokens, TokenStreamExt};

use heck::SnakeCase;

use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{bracketed, Ident, Result, Token};

mod provide_keywords {
    syn::custom_keyword!(from);
}

#[derive(Clone, Debug)]
pub struct Provide {
    pub(crate) provider: Ident,

    pub(crate) capabilities: Vec<Capability>,
}

#[derive(Clone, Debug)]
pub struct Capability {
    pub(crate) provider: Ident,

    pub(crate) name: Ident,
}

#[derive(Clone, Debug)]
pub struct Interface {
    pub(crate) capabilities: Vec<Ident>,
}

impl Parse for Provide {
    fn parse(input: ParseStream) -> Result<Self> {
        let provider: Ident = input.parse()?;
        let _: Token![=>] = input.parse()?;

        let content;
        let _ = bracketed!(content in input);
        let capabilities = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?
            .iter()
            .map(|name| Capability {
                provider: provider.clone(),
                name: name.clone(),
            })
            .collect();

        Ok(Provide {
            provider,

            capabilities,
        })
    }
}

impl Parse for Interface {
    fn parse(input: ParseStream) -> Result<Self> {
        let capabilities = Punctuated::<Ident, Token![,]>::parse_terminated(&input)?
            .iter()
            .cloned()
            .collect();

        Ok(Interface { capabilities })
    }
}

impl ToTokens for Provide {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Provide {
            provider,
            capabilities,
        } = self;

        let var_name = Ident::new(
            provider.to_string().as_str().to_snake_case().as_str(),
            provider.span(),
        );
        let deregister_capabilities: Vec<_> = capabilities
            .iter()
            .map(|capability| {
                let Capability { provider: _, name } = capability;

                let capability_name = Ident::new(
                    name.to_string().as_str().to_snake_case().as_str(),
                    name.span(),
                );
                let capability_name_str = format!("{}", capability_name);

                let error_str = format!("Couldn't deregister {}", capability_name);

                quote! {
                    registry_client.send(::registry::Deregister {
                        capability: #capability_name_str.to_string(),
                    })
                    .await
                    .expect(#error_str);
                }
            })
            .collect();

        let stream = quote! {
            async fn register_providers() -> ::core::result::Result<::actix::Addr<#provider>, ::failure::Error> {
                use ::registry::actix::*;

                let #var_name = #provider::start_default();
                let registry_client = ::registry::ProviderClient::connect_default().await?;

                #(#capabilities)*

                Ok(#var_name)
            }

            async fn deregister_providers() {
                use ::registry::actix::*;

                let registry_client = ::registry::ProviderClient::connect_default()
                    .await
                    .expect("Couldn't connect with registry to deregister capabilities");

                #(#deregister_capabilities)*
            }
        };

        tokens.append_all(stream);
    }
}

impl ToTokens for Capability {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Capability { provider, name } = self;

        let var_name = Ident::new(
            provider.to_string().as_str().to_snake_case().as_str(),
            provider.span(),
        );

        let capability_name = Ident::new(
            name.to_string().as_str().to_snake_case().as_str(),
            name.span(),
        );
        let capability_name_str = format!("{}", capability_name);

        let request_type = Ident::new(format!("{}Request", name).as_str(), name.span());

        let stream = quote! {
            ::registry::actix::Arbiter::spawn(Box::pin({
                let #var_name = #var_name.clone();
                let registry_client = registry_client.clone();

                async move {
                    let path = format!("/tmp/central.{}.{}", #capability_name_str, ::registry::uuid::Uuid::new_v4());
                    ::registry::cliff::server::IpcServer::<#request_type, #provider>::serve(path.as_str(), #var_name)
                        .expect("Couldn't start server for capability: #capability");
                    registry_client.send(::registry::Register {
                        capability: #capability_name_str.to_string(),
                        address: path,
                    })
                    .await
                    .expect(format!("Error sending Regiter message for {}", #capability_name_str).as_str());
                }
            }));
        };

        tokens.append_all(stream)
    }
}

impl ToTokens for Interface {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Interface { capabilities } = self;

        let impls: Vec<_> = capabilities
            .iter()
            .map(|capability| {
                let client_name =
                    Ident::new(format!("{}Client", capability).as_str(), capability.span());
                let capability_name = Ident::new(
                    capability.to_string().as_str().to_snake_case().as_str(),
                    capability.span(),
                );
                let capability_name_str = format!("{}", capability_name);

                quote! {
                    impl RegistryRequireableCapability for #client_name {
                        fn get_capability_name() -> String {
                            format!("{}", #capability_name_str)
                        }
                    }
                }
            })
            .collect();

        let streams = quote! {
            trait RegistryRequireableCapability {
                fn get_capability_name() -> String;
            }

            #(#impls)*

            async fn require<T: ::registry::cliff::client::IpcClient + RegistryRequireableCapability>() -> ::core::result::Result<::registry::actix::Addr<T>, ::registry::failure::Error> {
                let interface_client = ::registry::InterfaceClient::connect_default().await?;
                let path = interface_client
                    .send(::registry::Require { capability: T::get_capability_name() })
                    .await?;

                T::connect(path.as_str()).await
            }
        };

        tokens.append_all(streams);
    }
}
