use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, bracketed, token, Ident, Result, Token};

use super::nodes::*;

mod store_keywords {
    syn::custom_keyword!(command);
    syn::custom_keyword!(query);
    syn::custom_keyword!(into);
}

impl Parse for Ipc {
    fn parse(input: ParseStream) -> Result<Self> {
        let model_name: Ident = input.parse()?;

        let content;
        let _ = braced!(content in input);
        let fields: Vec<_> = Punctuated::<StoreField, Token![,]>::parse_terminated(&content)?
            .iter()
            .cloned()
            .collect();

        let query_actions = match fields.iter().find_map(|field| match field {
            StoreField::Query { actions } => Some(actions),
            _ => None,
        }) {
            Some(actions) => actions.to_vec(),
            None => {
                return Err(syn::Error::new(
                    model_name.span(),
                    "`query` field not found",
                ))
            }
        };

        let command_actions = match fields.iter().find_map(|field| match field {
            StoreField::Command { actions } => Some(actions),
            _ => None,
        }) {
            Some(actions) => actions.to_vec(),
            None => {
                return Err(syn::Error::new(
                    model_name.span(),
                    "`command` field not found",
                ))
            }
        };

        Ok(Ipc {
            model_name,
            query_actions,
            command_actions,
        })
    }
}

impl Parse for StoreField {
    fn parse(input: ParseStream) -> Result<Self> {
        use StoreField::*;
        let lookahead = input.lookahead1();

        let case = if lookahead.peek(store_keywords::command) {
            let _: store_keywords::command = input.parse()?;
            let _: Token![=>] = input.parse()?;

            let content;
            let _ = bracketed!(content in input);
            let actions: Vec<_> = Punctuated::<_, Token![,]>::parse_terminated(&content)?
                .iter()
                .cloned()
                .collect();

            Command { actions }
        } else if lookahead.peek(store_keywords::query) {
            let _: store_keywords::query = input.parse()?;
            let _: Token![=>] = input.parse()?;

            let content;
            let _ = bracketed!(content in input);
            let actions: Vec<_> = Punctuated::<_, Token![,]>::parse_terminated(&content)?
                .iter()
                .cloned()
                .collect();

            Query { actions }
        } else {
            return Err(lookahead.error());
        };

        Ok(case)
    }
}

impl Parse for CommandAction {
    fn parse(input: ParseStream) -> Result<Self> {
        let action_name = input.parse()?;

        let lookahead = input.lookahead1();

        let fields = if lookahead.peek(token::Brace) {
            Some(input.parse()?)
        } else if lookahead.peek(Token![->]) {
            None
        } else {
            return Err(lookahead.error());
        };

        let _: Token![->] = input.parse()?;
        let block = input.parse()?;

        Ok(CommandAction {
            action_name,
            fields,
            block,
        })
    }
}

impl Parse for QueryAction {
    fn parse(input: ParseStream) -> Result<Self> {
        let action_name = input.parse()?;

        let lookahead = input.lookahead1();

        let action_fields = if lookahead.peek(token::Brace) {
            Some(input.parse()?)
        } else if lookahead.peek(Token![->]) {
            None
        } else {
            return Err(lookahead.error());
        };

        let _: Token![->] = input.parse()?;

        let run_block = input.parse()?;
        let _: store_keywords::into = input.parse()?;
        let response_name = input.parse()?;

        let content;
        let _ = braced!(content in input);
        let response_fields: Vec<_> = Punctuated::<_, Token![,]>::parse_terminated(&content)?
            .iter()
            .cloned()
            .collect();

        let _: Token![=>] = input.parse()?;

        let result_block = input.parse()?;

        let _: Token![as] = input.parse()?;
        let result_type = input.parse()?;

        Ok(QueryAction {
            action_name,
            action_fields,

            run_block,

            response_name,
            response_fields,

            result_block,
            result_type,
        })
    }
}

impl Parse for QueryResponseField {
    fn parse(input: ParseStream) -> Result<Self> {
        let field_name = input.parse()?;
        let lookahead = input.lookahead1();

        let ty = if lookahead.peek(Token![:]) {
            let _: Token![:] = input.parse()?;
            Some(input.parse()?)
        } else if lookahead.peek(Token![=]) {
            None
        } else {
            return Err(lookahead.error());
        };

        let _: Token![=] = input.parse()?;
        let value = input.parse()?;

        Ok(QueryResponseField {
            field_name,
            ty,
            value,
        })
    }
}
