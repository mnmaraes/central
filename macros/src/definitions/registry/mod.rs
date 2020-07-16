use quote::{quote, ToTokens, TokenStreamExt};

use heck::SnakeCase;

use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, bracketed, token, Block, Expr, Ident, Result, Stmt, Token};

mod provide_keywords {
    syn::custom_keyword!(from);
    syn::custom_keyword!(setup);
    syn::custom_keyword!(provider);
}

#[derive(Clone, Debug)]
pub struct Provide {
    pub(crate) provider: Ident,

    pub(crate) setup: Option<Block>,
    pub(crate) create_provider: Option<Expr>,

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

#[derive(Clone, Debug)]
pub enum OptionField {
    Setup(Block),
    CreateProvider(Box<Expr>),
}

impl Parse for Provide {
    fn parse(input: ParseStream) -> Result<Self> {
        let provider: Ident = input.parse()?;

        let lookahead = input.lookahead1();
        let (setup, create_provider) = if lookahead.peek(token::Brace) {
            Provide::parse_options(input)?
        } else if lookahead.peek(Token![=>]) {
            (None, None)
        } else {
            return Err(lookahead.error());
        };

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

            setup,
            create_provider,

            capabilities,
        })
    }
}

impl Provide {
    fn parse_options(input: ParseStream) -> Result<(Option<Block>, Option<Expr>)> {
        let content;
        let _: token::Brace = braced!(content in input);
        let opts: Vec<OptionField> =
            Punctuated::<OptionField, Token![,]>::parse_terminated(&content)?
                .iter()
                .cloned()
                .collect();

        let setup = opts
            .iter()
            .find_map(|opt| match opt {
                OptionField::Setup(stmts) => Some(stmts),
                _ => None,
            })
            .cloned();

        let create_provider = opts.iter().find_map(|opt| match opt {
            OptionField::CreateProvider(expr) => Some(*(expr.clone())),
            _ => None,
        });

        Ok((setup, create_provider))
    }
}

impl Parse for OptionField {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(provide_keywords::setup) {
            let _: provide_keywords::setup = input.parse()?;
            let _: Token![=>] = input.parse()?;

            Ok(OptionField::Setup(input.parse()?))
        } else if lookahead.peek(provide_keywords::provider) {
            let _: provide_keywords::provider = input.parse()?;
            let _: Token![=>] = input.parse()?;

            Ok(OptionField::CreateProvider(input.parse()?))
        } else {
            Err(lookahead.error())
        }
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
            setup,
            create_provider,
        } = self;

        let var_name = Ident::new(
            provider.to_string().as_str().to_snake_case().as_str(),
            provider.span(),
        );

        let setup = match setup {
            Some(block) => {
                let stmts: Vec<_> = block.stmts.to_vec();
                quote! { #(#stmts;)* }
            }
            None => quote! {},
        };

        let create_provider = match create_provider {
            Some(expr) => quote! { #expr },
            None => quote! { #provider::start_default() },
        };

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

                #setup

                let #var_name = #create_provider;
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
                    .await??;

                T::connect(path.as_str()).await
            }
        };

        tokens.append_all(streams);
    }
}
