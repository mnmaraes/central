use super::nodes::*;

use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, bracketed, parenthesized, token, Ident, Result, Token, Type};

impl Parse for ServerInterface {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(token::Bracket) {
            let content;
            let _brackets = bracketed!(content in input);
            Ok(ServerInterface::Multiple(Punctuated::<
                ServerMessage,
                Token![,],
            >::parse_terminated(
                &content
            )?))
        } else if lookahead.peek(Ident) {
            Ok(ServerInterface::Single(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl Parse for CaseField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let _: Token![:] = input.parse()?;
        let ty = input.parse()?;

        Ok(CaseField { name, ty })
    }
}

impl Parse for RequestCase {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident = input.parse()?;
        let lookahead = input.lookahead1();

        let fields = if lookahead.peek(token::Brace) {
            let content;
            let _ = braced!(content in input);
            Punctuated::<CaseField, Token![,]>::parse_terminated(&content)?
                .iter()
                .cloned()
                .collect()
        } else if lookahead.peek(Token![->]) || lookahead.peek(Token![=>]) {
            vec![]
        } else {
            return Err(lookahead.error());
        };

        Ok(RequestCase { ident, fields })
    }
}

impl Parse for CaseFieldValue {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let lookahead = input.lookahead1();

        let value = if lookahead.peek(Token![:]) {
            let _: Token![:] = input.parse()?;
            Some(input.parse()?)
        } else {
            None
        };

        Ok(CaseFieldValue { name, value })
    }
}

impl Parse for ResponseCase {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let lookahead = input.lookahead1();

        let response = if lookahead.peek(token::Bracket) {
            let content;
            let _: token::Bracket = bracketed!(content in input);
            let types = Punctuated::<Type, Token![,]>::parse_terminated(&content)?
                .iter()
                .cloned()
                .collect();

            ResponseCase::Typed {
                name,
                types,
                build: Self::parse_build(input)?,
            }
        } else if lookahead.peek(token::Brace) {
            ResponseCase::Structured {
                name,
                build: Self::parse_build(input)?,
            }
        } else {
            ResponseCase::Empty { name }
        };

        Ok(response)
    }
}

impl ResponseCase {
    fn parse_build(input: ParseStream) -> Result<Punctuated<CaseFieldValue, Token![,]>> {
        let content;
        let _: token::Brace = braced!(content in input);

        Punctuated::parse_terminated(&content)
    }
}

impl Parse for ConditionalResponse {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();

        let cond = if lookahead.peek(Token![=>]) {
            None
        } else {
            let cond = input.parse()?;
            Some(cond)
        };
        let _: Token![=>] = input.parse()?;

        let response = input.parse()?;

        Ok(ConditionalResponse { cond, response })
    }
}

impl Parse for Response {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        let response = if lookahead.peek(token::Bracket) {
            Self::parse_conditional(input)?
        } else if lookahead.peek(Ident) {
            Self::parse_base(input)?
        } else {
            return Err(lookahead.error());
        };

        Ok(response)
    }
}

impl Response {
    fn parse_base(input: ParseStream) -> Result<Self> {
        Ok(Response::Base {
            case: input.parse()?,
        })
    }

    fn parse_conditional(input: ParseStream) -> Result<Self> {
        let content;
        let _: token::Bracket = bracketed!(content in input);

        Ok(Response::Conditional {
            cases: Punctuated::<ConditionalResponse, Token![,]>::parse_terminated(&content)?
                .iter()
                .cloned()
                .collect(),
        })
    }
}

impl Parse for RequestHandler {
    fn parse(input: ParseStream) -> Result<Self> {
        let request_case = input.parse()?;

        let lookahead = input.lookahead1();
        let block = if lookahead.peek(Token![->]) {
            let _: Token![->] = input.parse()?;
            Some(input.parse()?)
        } else if lookahead.peek(Token![=>]) {
            None
        } else {
            return Err(lookahead.error());
        };

        let _: Token![=>] = input.parse()?;
        let response = input.parse()?;

        Ok(RequestHandler {
            request_case,
            block,
            response,
        })
    }
}

impl Parse for ServerMessage {
    fn parse(input: ParseStream) -> Result<Self> {
        let interface_name = input.parse()?;

        let content;
        let _: token::Bracket = bracketed!(content in input);

        let handlers = Punctuated::<RequestHandler, Token![,]>::parse_terminated(&content)
            .iter()
            .flatten()
            .cloned()
            .collect();

        Ok(ServerMessage {
            interface_name,
            handlers,
        })
    }
}

impl Parse for Router {
    fn parse(input: ParseStream) -> Result<Self> {
        let router_type: Ident = input.parse()?;

        let lookahead = input.lookahead1();
        let interface = if lookahead.peek(Token![;]) {
            let _: Token![;] = input.parse()?;
            input.parse()?
        } else if lookahead.peek(token::Bracket) {
            let content;
            let _: token::Bracket = bracketed!(content in input);
            let handlers = Punctuated::<RequestHandler, Token![,]>::parse_terminated(&content)
                .iter()
                .flatten()
                .cloned()
                .collect();

            ServerInterface::Single(ServerMessage {
                interface_name: router_type.clone(),
                handlers,
            })
        } else {
            return Err(lookahead.error());
        };

        Ok(Router {
            router_type,

            interface,
        })
    }
}

mod client_keywords {
    syn::custom_keyword!(named);
    syn::custom_keyword!(into);
    syn::custom_keyword!(wait);
    syn::custom_keyword!(on);
    syn::custom_keyword!(actions);
    syn::custom_keyword!(response_mapping);
}

impl Parse for Client {
    fn parse(input: ParseStream) -> Result<Self> {
        let request_type: Ident = input.parse()?;
        let lookahead = input.lookahead1();

        let client_name = if lookahead.peek(client_keywords::named) {
            let _: client_keywords::named = input.parse()?;
            input.parse()?
        } else if lookahead.peek(token::Brace) {
            request_type.clone()
        } else {
            return Err(lookahead.error());
        };

        let content;
        let _: token::Brace = braced!(content in input);

        let interface: Vec<ClientFields> =
            Punctuated::<ClientFields, Token![,]>::parse_terminated(&content)?
                .iter()
                .cloned()
                .collect();

        let actions = match interface.iter().find(|field| match field {
            ClientFields::Actions(_) => true,
            _ => false,
        }) {
            Some(ClientFields::Actions(actions)) => actions.to_vec(),
            _ => {
                return Err(syn::Error::new(
                    request_type.span(),
                    "Missing 'actions' field",
                ))
            }
        };

        let response_mapping = match interface.iter().find(|field| match field {
            ClientFields::ResponseMapping(_) => true,
            _ => false,
        }) {
            Some(ClientFields::ResponseMapping(mapping)) => mapping.to_vec(),
            _ => {
                return Err(syn::Error::new(
                    request_type.span(),
                    "Missing 'actions' field",
                ))
            }
        };

        Ok(Client {
            request_type,
            client_name,

            actions,
            response_mapping,
        })
    }
}

impl Parse for ClientFields {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        let case = if lookahead.peek(client_keywords::actions) {
            let _: client_keywords::actions = input.parse()?;
            let _: Token![=>] = input.parse()?;

            let content;
            let _ = bracketed!(content in input);
            let actions = Punctuated::<ClientAction, Token![,]>::parse_terminated(&content)?
                .iter()
                .cloned()
                .collect();

            ClientFields::Actions(actions)
        } else if lookahead.peek(client_keywords::response_mapping) {
            let _: client_keywords::response_mapping = input.parse()?;
            let _: Token![=>] = input.parse()?;

            let content;
            let _ = bracketed!(content in input);
            let mappings = Punctuated::<ResponseMapping, Token![,]>::parse_terminated(&content)?
                .iter()
                .cloned()
                .collect();

            ClientFields::ResponseMapping(mappings)
        } else {
            return Err(lookahead.error());
        };

        Ok(case)
    }
}

impl Parse for ClientAction {
    fn parse(input: ParseStream) -> Result<Self> {
        let action_type = input.parse()?;
        let lookahead = input.lookahead1();

        let mapped_request = if lookahead.peek(client_keywords::into) {
            let _: client_keywords::into = input.parse()?;
            Some(input.parse()?)
        } else {
            None
        };

        let lookahead = input.lookahead1();
        let response = if lookahead.peek(client_keywords::wait) {
            Some(input.parse()?)
        } else {
            None
        };

        Ok(ClientAction {
            action_type,

            mapped_request,
            response,
        })
    }
}

impl Parse for ActionType {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let lookahead = input.lookahead1();

        let fields = if lookahead.peek(token::Brace) {
            let content;
            let _ = braced!(content in input);
            Punctuated::<CaseField, Token![,]>::parse_terminated(&content)?
                .iter()
                .cloned()
                .collect()
        } else if lookahead.peek(client_keywords::wait)
            || lookahead.peek(client_keywords::into)
            || lookahead.peek(Token![,])
        {
            vec![]
        } else {
            return Err(lookahead.error());
        };

        Ok(ActionType { name, fields })
    }
}

impl Parse for ActionMapping {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        let case = if lookahead.peek(Ident) {
            ActionMapping::ExprMapping(Box::new(input.parse()?))
        } else if lookahead.peek(token::Brace) {
            ActionMapping::BlockMapping(input.parse()?)
        } else {
            return Err(lookahead.error());
        };

        Ok(case)
    }
}

impl Parse for TypedActionMapping {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        let case = if lookahead.peek(Ident) {
            TypedActionMapping::parse_typed_case(input)?
        } else if lookahead.peek(token::Paren) {
            TypedActionMapping::parse_unit_case(input)?
        } else {
            return Err(lookahead.error());
        };

        Ok(case)
    }
}

impl TypedActionMapping {
    fn parse_typed_case(input: ParseStream) -> Result<Self> {
        let ty = Box::new(input.parse()?);
        let _: Token![:] = input.parse()?;

        let lookahead = input.lookahead1();
        let case = if lookahead.peek(Ident) {
            TypedActionMapping::ExprMapping {
                ty,
                expr: Box::new(input.parse()?),
            }
        } else if lookahead.peek(token::Brace) {
            TypedActionMapping::BlockMapping {
                ty,
                block: input.parse()?,
            }
        } else {
            return Err(lookahead.error());
        };

        Ok(case)
    }

    fn parse_unit_case(input: ParseStream) -> Result<Self> {
        let _content;
        let _ = parenthesized!(_content in input);

        let lookahead = input.lookahead1();
        let block = if lookahead.peek(Token![:]) {
            Some(input.parse()?)
        } else {
            None
        };

        Ok(TypedActionMapping::UnitMapping { block })
    }
}

impl Parse for ClientResponse {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        let case = if lookahead.peek(client_keywords::wait) {
            let _: client_keywords::wait = input.parse()?;
            ClientResponse::Wait(input.parse()?)
        } else {
            return Err(lookahead.error());
        };

        Ok(case)
    }
}

impl Parse for WaitResponse {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        let ty = if lookahead.peek(Token![,]) {
            None
        } else if lookahead.peek(Ident) {
            Some(input.parse()?)
        } else {
            return Err(lookahead.error());
        };

        Ok(WaitResponse { ty })
    }
}

impl Parse for ResponseMapping {
    fn parse(input: ParseStream) -> Result<Self> {
        let response_case = input.parse()?;
        let _: Token![=>] = input.parse()?;

        let content;
        let _ = bracketed!(content in input);
        let action_mapping =
            Punctuated::<TypedActionMapping, Token![,]>::parse_terminated(&content)?
                .iter()
                .cloned()
                .collect();

        Ok(ResponseMapping {
            response_case,
            action_mapping,
        })
    }
}

impl Parse for ResponseMappingCase {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let lookahead = input.lookahead1();

        let response = if lookahead.peek(token::Brace) {
            let content;
            let _: token::Brace = braced!(content in input);

            let build = Punctuated::parse_terminated(&content)?;
            ResponseMappingCase::Structured { name, build }
        } else {
            ResponseMappingCase::Empty { name }
        };

        Ok(response)
    }
}
