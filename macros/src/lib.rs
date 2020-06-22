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

#[cfg(feature = "store")]
use definitions::store::Ipc;

#[cfg(feature = "cliff")]
#[proc_macro]
pub fn client(tokens: TokenStream) -> TokenStream {
    let client = parse_macro_input!(tokens as Client);

    TokenStream::from(quote! { #client })
}

#[cfg(feature = "cliff")]
#[proc_macro]
pub fn router(tokens: TokenStream) -> TokenStream {
    let router = parse_macro_input!(tokens as Router);

    let expanded = build_router(router);

    TokenStream::from(expanded)
}

#[cfg(feature = "registry")]
#[proc_macro]
pub fn interface(tokens: TokenStream) -> TokenStream {
    let interface = parse_macro_input!(tokens as Interface);

    TokenStream::from(quote! { #interface })
}

#[cfg(feature = "registry")]
#[proc_macro]
pub fn provide(tokens: TokenStream) -> TokenStream {
    let provide = parse_macro_input!(tokens as Provide);

    TokenStream::from(quote! { #provide })
}

#[cfg(feature = "registry")]
#[proc_macro]
pub fn run_provide(tokens: TokenStream) -> TokenStream {
    let provide = parse_macro_input!(tokens as Provide);
    let ident = provide.provider.clone();

    let file_name = format!("{}.log", ident);

    TokenStream::from(quote! {
        #provide

        fn main() -> Result<(), ::registry::failure::Error> {
            ::registry::actix_rt::System::new("main").block_on(async move {
                dotenv::dotenv()?;
                let log_dir = std::env::var("LOG_DIRECTORY")?;

                let file_appender = ::registry::tracing_appender::rolling::daily(&log_dir, #file_name);
                let (non_blocking, _guard) = ::registry::tracing_appender::non_blocking(file_appender);
                ::registry::tracing_subscriber::fmt().with_writer(non_blocking).init();

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

#[cfg(feature = "store")]
#[proc_macro]
pub fn ipc(tokens: TokenStream) -> TokenStream {
    let ipc = parse_macro_input!(tokens as Ipc);

    TokenStream::from(quote! { #ipc })
}

#[cfg(feature = "store")]
#[proc_macro]
pub fn provide_store(tokens: TokenStream) -> TokenStream {
    let name = parse_macro_input!(tokens as syn::Ident);

    let store_name = quote::format_ident!("{}Store", name);
    let command_name = quote::format_ident!("{}Command", name);
    let query_name = quote::format_ident!("{}Query", name);
    let status_name = quote::format_ident!("{}Status", store_name);

    let command_request_name = quote::format_ident!("{}CommandRequest", name);
    let query_request_name = quote::format_ident!("{}QueryRequest", name);
    let status_request_name = quote::format_ident!("{}StatusRequest", store_name);

    TokenStream::from(quote! {
        #[macro_use]
        extern crate diesel;

        mod ipc;
        mod models;

        use registry::run_provide;

        use ipc::{#store_name, #command_request_name, #query_request_name, #status_request_name};

        run_provide! {
            #store_name => [#command_name, #status_name, #query_name]
        }
    })
}
