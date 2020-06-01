use quote::{quote, ToTokens, TokenStreamExt};

use syn::Ident;

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
