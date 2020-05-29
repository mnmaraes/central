extern crate proc_macro;

use proc_macro2::TokenStream;

#[proc_macro]
pub fn server(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let tokens = TokenStream::from(tokens);

    proc_macro::TokenStream::from(tokens)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
