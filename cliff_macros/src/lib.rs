extern crate proc_macro;

mod definitions;
mod utils;

use quote::quote;

use syn::parse_macro_input;

use definitions::{Client, Router};
use utils::build_router;

#[proc_macro]
pub fn client(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let client = parse_macro_input!(tokens as Client);

    proc_macro::TokenStream::from(quote! { #client })
}

#[proc_macro]
pub fn router(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let router = parse_macro_input!(tokens as Router);

    let expanded = build_router(router);

    proc_macro::TokenStream::from(expanded)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
