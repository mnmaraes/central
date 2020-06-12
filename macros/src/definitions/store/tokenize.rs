use itertools::Itertools;

use quote::{format_ident, quote, ToTokens, TokenStreamExt};

use super::nodes::*;

impl ToTokens for Ipc {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Ipc {
            model_name,
            command_actions,
            query_actions,
        } = self;

        let store_name = format_ident!("{}Store", model_name);

        let command_name = format_ident!("{}Command", model_name);
        let query_name = format_ident!("{}Query", model_name);
        let status_name = format_ident!("{}Status", store_name);

        let (cmd_router_actions, cmd_client_handlers): (Vec<_>, Vec<_>) = command_actions
            .iter()
            .map(|act| (act.to_router_tokens(), act.to_client_tokens()))
            .unzip();

        let (qry_router_actions, qry_client_handlers): (Vec<_>, Vec<_>) = query_actions
            .iter()
            .map(|act| (act.to_router_tokens(), act.to_client_tokens()))
            .unzip();

        let qry_response_mappings: Vec<_> = get_response_mappings(query_actions);
        let query_response_types: Vec<_> = query_actions
            .iter()
            .map(|act| act.result_type.clone())
            .unique()
            .collect();

        let stream = quote! {
            mod ipc {
                use super::*;

                use failure::{format_err, Error};

                use ::actix::prelude::*;
                use ::diesel::prelude::*;

                pub struct #store_name {
                    connection: PgConnection,
                }

                impl Default for #store_name {
                    fn default() -> Self {
                        ::dotenv::dotenv().expect("Unable to load environment");

                        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL env var not found");

                        let connection = PgConnection::establish(&database_url)
                            .unwrap_or_else(|_| panic!("Couldn't connect to {}", database_url));

                        #store_name { connection }
                    }
                }

                impl Actor for #store_name {
                    type Context = Context<Self>;
                }

                ::cliff::router! {
                    #store_name;
                    [
                        #command_name [
                            #(#cmd_router_actions),*
                        ],
                        #query_name [
                            #(#qry_router_actions),*
                        ],
                        #status_name [
                            Check => Alive
                        ]
                    ]
                }

                ::cliff::client! {
                    #command_name {
                        actions => [
                            #(#cmd_client_handlers),*
                        ],
                        response_mapping => [
                            Success => [ () ],
                            Error => [ () ]
                        ]
                    }
                }

                ::cliff::client! {
                    #query_name {
                        actions => [
                            #(#qry_client_handlers),*
                        ],
                        response_mapping => [
                            #(#qry_response_mappings,)*
                            Error { description  } => [
                                #(Result<#query_response_types, Error>: Err(format_err!("{}",description))),*
                            ]
                        ]
                    }
                }

                ::cliff::client! {
                    #status_name {
                        actions => [ Check wait, ],
                        response_mapping => [ Alive => [ () ] ]
                    }
                }
            }
        };

        tokens.append_all(stream)
    }
}

impl CommandAction {
    fn to_router_tokens(&self) -> proc_macro2::TokenStream {
        let CommandAction {
            action_name,
            fields,
            block,
        } = self;

        quote! {
            #action_name #fields -> {
                let res = #block;
            } => [
                let Err(e) = res => Error,
                => Success
            ]
        }
    }

    fn to_client_tokens(&self) -> proc_macro2::TokenStream {
        let CommandAction {
            action_name,
            fields,
            ..
        } = self;

        quote! { #action_name #fields wait }
    }
}

impl QueryAction {
    fn to_router_tokens(&self) -> proc_macro2::TokenStream {
        let QueryAction {
            action_name,
            action_fields,
            run_block,
            response_name,
            response_fields,
            ..
        } = self;

        let field_types: Vec<_> = response_fields
            .iter()
            .filter_map(|field| field.ty.clone())
            .collect();

        let type_decls = if field_types.is_empty() {
            None
        } else {
            Some(quote! { [#(#field_types),*] })
        };

        let field_values = if response_fields.is_empty() {
            None
        } else {
            let fields: Vec<_> = response_fields
                .iter()
                .map(|f| {
                    let QueryResponseField {
                        field_name, value, ..
                    } = f;
                    quote! { #field_name: #value }
                })
                .collect();

            Some(quote! { {#(#fields),*} })
        };

        quote! {
            #action_name #action_fields -> {
                let result = #run_block;
            } => [
                let Err(e) = result => Error [String] { description: format!("{}", e) },
                => #response_name #type_decls #field_values
            ]
        }
    }

    fn to_client_tokens(&self) -> proc_macro2::TokenStream {
        let QueryAction {
            action_name,
            action_fields,
            result_type,
            ..
        } = self;

        quote! { #action_name #action_fields wait Result<#result_type, Error> }
    }

    fn to_result_mapping(&self) -> ResultMapping {
        let QueryAction {
            response_name,
            response_fields,
            ..
        } = self;

        let field_names = if response_fields.is_empty() {
            None
        } else {
            let fields: Vec<_> = response_fields
                .iter()
                .map(|f| f.field_name.clone())
                .collect();

            Some(quote! { {#(#fields),*} })
        };

        ResultMapping {
            response_name: response_name.clone(),
            field_names: quote! { #field_names },
        }
    }
}

impl ToTokens for ResultMapping {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ResultMapping {
            response_name,
            field_names,
        } = self;

        let stream = quote! { #response_name #field_names };

        tokens.append_all(stream);
    }
}

impl std::cmp::PartialEq for ResultMapping {
    #[allow(clippy::cmp_owned)]
    fn eq(&self, other: &Self) -> bool {
        self.response_name.to_string() == other.response_name.to_string()
    }
}

impl std::cmp::Eq for ResultMapping {}

impl std::hash::Hash for ResultMapping {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.response_name.to_string().hash(state);
    }
}

fn get_response_mappings(vec: &[QueryAction]) -> Vec<proc_macro2::TokenStream> {
    vec.iter()
        .map(|act| {
            (
                act.to_result_mapping(),
                (act.result_type.clone(), act.result_block.clone()),
            )
        })
        .into_group_map()
        .iter()
        .map(|(response, mappings)| {
            let mappings: Vec<_> = mappings
                .iter()
                .map(|(ty, block)| quote! { Result<#ty, Error>: Ok(#block) })
                .collect();

            quote! {
                #response => [
                    #(#mappings),*
                ]
            }
        })
        .collect()
}
