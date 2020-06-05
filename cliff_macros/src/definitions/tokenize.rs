use quote::{quote, ToTokens, TokenStreamExt};

use syn::{Ident, Index};

use super::nodes::*;

impl ToTokens for CaseField {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let CaseField { name, ty } = self;
        tokens.append_all(quote! { #name: #ty })
    }
}

impl ToTokens for RequestCase {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let RequestCase { ident, fields } = self;
        let stream = if fields.is_empty() {
            quote! { #ident }
        } else {
            let field_names: Vec<Ident> = fields.iter().map(|f| f.name.clone()).collect();
            quote! { #ident { #(#field_names),* } }
        };

        tokens.append_all(stream);
    }
}

impl ToTokens for CaseFieldValue {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let CaseFieldValue { name, value } = self;
        let stream = if let Some(value) = value {
            quote! { #name: #value }
        } else {
            quote! { #name }
        };

        tokens.append_all(stream);
    }
}

impl ToTokens for CaseDeclaration {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let CaseDeclaration { name, fields } = self;
        let stream = if !fields.is_empty() {
            quote! {
                #name {
                    #(#fields),*
                }
            }
        } else {
            quote! { #name }
        };

        tokens.append_all(stream);
    }
}

impl ToTokens for ResponseCase {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        use ResponseCase::*;

        let stream = match self {
            Empty { name } => quote! { #name },
            Structured { name, build } => quote! { #name { #build } },
            Typed {
                name,
                types: _,
                build,
            } => quote! { #name { #build } },
        };

        tokens.append_all(stream)
    }
}

impl ToTokens for Response {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let stream = match self {
            Response::Base { case } => quote! {
                #case
            },
            Response::Conditional { cases } => {
                let last_idx = cases.len() - 1;
                cases
                    .iter()
                    .enumerate()
                    .fold(quote! {}, |mut acc, (i, case)| {
                        let ConditionalResponse { cond, response } = case;
                        let stream = if i == 0 {
                            quote! {
                                if #cond {
                                    #response
                                }
                            }
                        } else if i == last_idx && cond == &None {
                            quote! {
                                else {
                                    #response
                                }
                            }
                        } else {
                            quote! {
                                else if #cond {
                                    #response
                                }
                            }
                        };

                        acc.append_all(stream);

                        acc
                    })
            }
        };

        tokens.append_all(stream)
    }
}

impl ToTokens for RequestHandler {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let RequestHandler {
            request_case,
            block,
            response,
        } = self;

        let stream = if let Some(block) = block {
            let stmts = block.stmts.clone();
            quote! {
                #request_case => {
                    #(#stmts);*
                    #response
                }
            }
        } else {
            quote! {
                #request_case => {
                    #response
                }
            }
        };

        tokens.append_all(stream);
    }
}

impl ToTokens for Client {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Client {
            request_type: _,
            client_name: _,
            actions,
            response_mapping,
        } = self;

        let client = self.get_client_type_name();

        let request_name = self.get_request_type_name();
        let response_name = self.get_response_type_name();

        let action_declarations: Vec<ActionDeclaration> =
            actions.iter().map(|action| action.into()).collect();

        let future_descriptors = FutureDescriptor::get_descriptors(self);

        let (futures_declaration, futures_init) = Client::get_futures(&future_descriptors);

        let handler_declarations = HandlerDeclaration::get_declarations(self, &future_descriptors);

        let response_mapping: Vec<FutureResponseMapping> = response_mapping
            .iter()
            .map(|mapping| FutureResponseMapping::wrap_mapping(mapping, &future_descriptors))
            .collect();

        let stream = quote! {
            pub struct #client {
                writer: actix::Addr<cliff::client::WriteInterface<#request_name>>,
                #futures_declaration
            }

            impl Actor for #client {
                type Context = Context<Self>;
            }

            #(#action_declarations)*

            #(#handler_declarations)*

            impl StreamHandler<::core::result::Result<#response_name, ::failure::Error>> for #client {
              fn handle(&mut self, item: Result<#response_name, ::failure::Error>, _ctx: &mut Self::Context) {
                use #response_name::*;

                match item {
                  #(#response_mapping),*
                  _ => {}
                }
              }
            }

            impl #client {
              pub async fn connect(path: &str) -> core::result::Result<Addr<Self>, ::failure::Error> {
                use ::failure::ResultExt;
                use ::cliff::client::Delegate;

                let stream = ::tokio::net::UnixStream::connect(path).await?;
                let (r, w) = ::tokio::io::split(stream);

                let writer = ::cliff::client::WriteInterface::<#request_name>::attach(w).await?;

                let addr = #client::create(|ctx| {
                  #client::listen(r, ctx);

                  #client {
                    writer,
                    #futures_init
                  }
                });

                Ok(addr)
              }
            }
        };

        tokens.append_all(stream)
    }
}

impl Client {
    fn get_futures(
        future_descriptors: &[FutureDescriptor],
    ) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
        if future_descriptors.is_empty() {
            (quote! {}, quote! {})
        } else if future_descriptors.len() == 1 {
            let descriptor = future_descriptors[0].clone();
            (
                quote! {
                    futures: #descriptor
                },
                quote! {
                    futures: ::std::colections::HashMap::new();
                },
            )
        } else {
            let hashes: Vec<proc_macro2::TokenStream> = future_descriptors
                .iter()
                .cloned()
                .map(|_| quote! { ::std::collections::HashMap::new() })
                .collect();

            (
                quote! {
                    futures: (#(#future_descriptors),*)
                },
                quote! {
                    futures: (#(#hashes),*)
                },
            )
        }
    }
}

impl ToTokens for ActionDeclaration {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ActionDeclaration {
            name,
            fields,
            result_type,
        } = self;

        let (long_decl, short_decl) = if let Some(ty) = result_type {
            (
                Some(quote! {
                    impl ::actix::Message for #name {
                        type Result = #ty;
                    }
                }),
                quote! {
                    #[derive(Debug)]
                },
            )
        } else {
            (
                None,
                quote! {
                    #[derive(Debug, Message)]
                    #[rtype(result = "()")]
                },
            )
        };

        let stream = if fields.is_empty() {
            quote! {
                #short_decl
                pub struct #name;

                #long_decl
            }
        } else {
            quote! {
                #short_decl
                pub struct #name {
                    #(pub #fields),*
                }

                #long_decl
            }
        };

        tokens.append_all(stream)
    }
}

impl ToTokens for FutureDescriptor {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let def = quote! { () };
        let result_type = self.result_type.clone().map_or(def, |ty| quote! { #ty });

        let stream = quote! { std::collections::HashMap<String, ::tokio::sync::oneshot::Sender<#result_type>> };

        tokens.append_all(stream);
    }
}

impl ToTokens for HandlerDeclaration {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let HandlerDeclaration {
            client_name,
            request_name,
            future_mapping,
            action:
                ClientAction {
                    action_type,
                    mapped_request,
                    response,
                },
        } = self;

        let action_name = action_type.name.clone();

        let response_type = response
            .clone()
            .and_then(|response| match response {
                ClientResponse::Wait(WaitResponse { ty: Some(ty) }) => Some(quote! { #ty }),
                _ => None,
            })
            .unwrap_or(quote! { () });

        let request_mapping = if let Some(mapping) = mapped_request {
            quote! { #mapping }
        } else if action_type.fields.is_empty() {
            let name = action_type.name.clone();
            quote! { #name }
        } else {
            quote! { #action_type }
        };

        let then_mapping = match future_mapping {
            FutureRequestMapping::None => quote! { .then(|_| async {}) },
            _ => quote! { .then(|_| async move { rx.await.unwrap() }) },
        };

        let stream = quote! {
          impl Handler<#action_name> for #client_name {
            type Result = ResponseFuture<#response_type>;

            fn handle(&mut self, msg: #action_name, _ctx: &mut Self::Context) -> Self::Result {
              use ::futures::FutureExt;
              use #request_name::*;

              #action_type;

              #future_mapping

              Box::pin(
                  self.writer
                      .send(::cliff::client::InterfaceRequest(#request_mapping))
                      #then_mapping
              )
            }
          }
        };

        tokens.append_all(stream)
    }
}

impl ToTokens for ActionType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ActionType { name, fields } = self;

        let stream = if fields.is_empty() {
            quote! {}
        } else {
            let field_names: Vec<Ident> = fields.iter().map(|f| f.name.clone()).collect();
            quote! {
              let #name { #(#field_names),* } = msg;
            }
        };

        tokens.append_all(stream);
    }
}

impl ToTokens for ActionMapping {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        use ActionMapping::*;

        let stream = match self {
            ExprMapping(expr) => quote! { #expr },
            BlockMapping(block) => quote! { #block },
        };

        tokens.append_all(stream);
    }
}

impl ToTokens for FutureRequestMapping {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let future = match self {
            FutureRequestMapping::Indexed(idx) => {
                let index = Index::from(*idx);
                quote! { futures.#index }
            }
            FutureRequestMapping::Single => quote! { futures },
            FutureRequestMapping::None => return,
        };

        let stream = quote! {
          let id = ::uuid::Uuid::new_v4();
          let (tx, rx) = ::tokio::sync::oneshot::channel();

          self.#future.insert(id.to_string(), tx);
        };

        tokens.append_all(stream)
    }
}

impl ToTokens for ResponseMappingCase {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        use ResponseMappingCase::*;

        let stream = match self {
            Empty { name } => quote! { #name },
            Structured { name, build } => quote! { #name { #build } },
        };

        tokens.append_all(stream)
    }
}

impl ToTokens for FutureResponseMapping {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let stream = match self {
            FutureResponseMapping::Single {
                mapping_case,
                action_mapping,
            } => quote! {
              Ok(#mapping_case) => {
                if let Some(tx) = self.futures.remove(&id) {
                  tx.send(#action_mapping).unwrap();
                }
              }
            },
            FutureResponseMapping::Indexed {
                mapping_case,
                indexed_mappings,
            } => quote! {
              Ok(#mapping_case) => {
                #(#indexed_mappings)*
              }
            },
        };

        tokens.append_all(stream)
    }
}

impl ToTokens for TypedActionMapping {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let stream = match self {
            TypedActionMapping::UnitMapping { block: Some(block) } => quote! { #block },
            TypedActionMapping::UnitMapping { block: None } => quote! { () },
            TypedActionMapping::ExprMapping { ty: _, expr } => quote! { #expr },
            TypedActionMapping::BlockMapping { ty: _, block } => quote! { #block },
        };

        tokens.append_all(stream);
    }
}

impl ToTokens for IndexMapping {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let IndexMapping {
            index,
            action_mapping,
        } = self;
        let index = Index::from(*index);

        let stream = quote! {
            if let Some(tx) = self.futures.#index.remove("") {
              tx.send(#action_mapping).unwrap();
            }
        };

        tokens.append_all(stream);
    }
}
