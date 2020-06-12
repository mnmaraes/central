extern crate proc_macro;

mod definitions;

use proc_macro::TokenStream;

use quote::quote;

use syn::parse_macro_input;

#[cfg(feature = "cliff")]
use definitions::cliff::{build_router, Client, Router};

#[cfg(feature = "registry")]
use definitions::registry::{Interface, Provide};

#[cfg(feature = "cli")]
use definitions::cli::Cli;

#[cfg(feature = "cliff")]
#[proc_macro]
pub fn client(tokens: TokenStream) -> TokenStream {
    let client = parse_macro_input!(tokens as Client);

    TokenStream::from(quote! { #client })
}

#[cfg(feature = "cliff")]
#[proc_macro]
pub fn router(tokens: proc_macro::TokenStream) -> TokenStream {
    let router = parse_macro_input!(tokens as Router);

    let expanded = build_router(router);

    TokenStream::from(expanded)
}

#[cfg(feature = "registry")]
#[proc_macro]
pub fn interface(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let interface = parse_macro_input!(tokens as Interface);

    proc_macro::TokenStream::from(quote! { #interface })
}

#[cfg(feature = "registry")]
#[proc_macro]
pub fn provide(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let provide = parse_macro_input!(tokens as Provide);

    proc_macro::TokenStream::from(quote! { #provide })
}

#[cfg(feature = "registry")]
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

                Ok(())
            })
        }
    })
}

#[cfg(feature = "cli")]
#[proc_macro]
pub fn cli(tokens: TokenStream) -> TokenStream {
    let cli = parse_macro_input!(tokens as Cli);

    TokenStream::from(quote! { #cli })
}
