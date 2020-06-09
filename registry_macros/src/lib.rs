extern crate proc_macro;

mod definitions;

use quote::quote;

use syn::parse_macro_input;

use definitions::{Interface, Provide};

#[proc_macro]
pub fn interface(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let interface = parse_macro_input!(tokens as Interface);

    proc_macro::TokenStream::from(quote! { #interface })
}

#[proc_macro]
pub fn provide(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let provide = parse_macro_input!(tokens as Provide);

    proc_macro::TokenStream::from(quote! { #provide })
}

#[proc_macro]
pub fn run_provide(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let provide = parse_macro_input!(tokens as Provide);

    proc_macro::TokenStream::from(quote! {
        #provide

        fn main() -> Result<(), ::registry::failure::Error> {
            ::registry::actix_rt::System::new("main").block_on(async move {
                let _: Result<(), ::registry::failure::Error> = {
                    register_providers().await?;

                    ::registry::tokio::signal::ctrl_c().await?;
                    Ok(())
                };

                deregister_providers().await;
                ::registry::actix::System::current().stop();

                println!("Shutting Down");

                Ok(())
            })
        }
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
