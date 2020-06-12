extern crate proc_macro;

mod definitions;

use proc_macro::TokenStream;

use quote::quote;

use syn::parse_macro_input;

use definitions::Cli;

#[proc_macro]
pub fn cli(tokens: TokenStream) -> TokenStream {
    let cli = parse_macro_input!(tokens as Cli);

    TokenStream::from(quote! { #cli })
}

