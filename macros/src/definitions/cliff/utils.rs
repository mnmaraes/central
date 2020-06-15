use std::convert::{From, TryFrom};

use itertools::Itertools;

use quote::quote;

use syn::Ident;

use super::nodes::{
    CaseDeclaration, RequestHandler, Response, Router, ServerInterface, ServerMessage,
};

pub fn build_router(router: Router) -> proc_macro2::TokenStream {
    use ServerInterface::*;

    let Router {
        router_type,
        interface,
    } = router;

    match interface {
        Single(message) => build_server_message(router_type, message),
        Multiple(ms) => {
            let messages = ms
                .iter()
                .cloned()
                .map(|m| build_server_message(router_type.clone(), m));

            quote! {
                #(#messages)*
            }
        }
    }
}

fn build_server_message(router_type: Ident, message: ServerMessage) -> proc_macro2::TokenStream {
    let request_type_name = Ident::new(
        &format!("{}Request", message.interface_name),
        proc_macro2::Span::call_site(),
    );
    let response_type_name = Ident::new(
        &format!("{}Response", message.interface_name),
        proc_macro2::Span::call_site(),
    );

    let (request_cases, response_cases) = build_declarations(&message.handlers);

    let handlers = message.handlers;

    quote! {
        #[derive(::cliff::serde::Serialize, ::cliff::serde::Deserialize, Debug)]
        #[serde(crate = "::cliff::serde")]
        pub enum #request_type_name {
            #request_cases
        }

        impl ::cliff::actix::Message for #request_type_name {
            type Result = #response_type_name;
        }

        #[derive(::cliff::serde::Serialize, ::cliff::serde::Deserialize, ::cliff::actix::Message, Debug)]
        #[rtype(result = "()")]
        #[serde(crate = "::cliff::serde")]
        pub enum #response_type_name {
            #response_cases
        }

        impl<A, M> ::cliff::actix::dev::MessageResponse<A, M> for #response_type_name
        where
            A: ::cliff::actix::Actor,
            M: ::cliff::actix::Message<Result = #response_type_name>,
        {
            fn handle<R: ::cliff::actix::dev::ResponseChannel<M>>(self, _: &mut A::Context, tx: Option<R>) {
                if let Some(tx) = tx {
                    tx.send(self);
                }
            }
        }

        impl Handler<#request_type_name> for #router_type {
            type Result = #response_type_name;

            fn handle(&mut self, msg: #request_type_name, _ctx: &mut Self::Context) -> Self::Result {
                use #request_type_name::*;
                use #response_type_name::*;

                match msg {
                    #(#handlers)*
                }
            }
        }

        impl ::cliff::server::Router<#request_type_name> for #router_type {}
    }
}

fn build_declarations(
    handlers: &[RequestHandler],
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let request_cases: Vec<CaseDeclaration> = handlers
        .iter()
        .map(|h| CaseDeclaration::from(&h.request_case))
        .collect();

    let response_cases: Vec<CaseDeclaration> = handlers
        .iter()
        .flat_map(|h| match h.response.clone() {
            Response::Base { case } => vec![case],
            Response::Conditional { cases } => cases.iter().map(|c| c.response.clone()).collect(),
        })
        .filter_map(|c| CaseDeclaration::try_from(&c).ok())
        .unique_by(|c| c.name.clone())
        .collect();

    (
        quote! { #(#request_cases),* },
        quote! { #(#response_cases),* },
    )
}
