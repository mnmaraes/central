use super::nodes::*;

use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, bracketed, token, Ident, Result, Token, Type};

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
