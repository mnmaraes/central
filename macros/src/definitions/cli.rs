use quote::{quote, ToTokens, TokenStreamExt};

use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{bracketed, Attribute, Ident, Result, Token};

#[derive(Clone, Debug)]
pub struct Cli {
    main_command: Command,
}

impl Parse for Cli {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Cli {
            main_command: input.parse()?,
        })
    }
}

impl ToTokens for Cli {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Cli { main_command } = self;

        let command_name = main_command.get_name();

        let stream = quote! {
            #main_command

            fn main() {
                #command_name::parse().run();
            }
        };

        tokens.append_all(stream);
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    Parent {
        name: Ident,
        attrs: Vec<Attribute>,
        children: Vec<Command>,
    },
    Runnable {
        name: Ident,
        attrs: Vec<Attribute>,
        runnable: Ident,
    },
}

impl Parse for Command {
    fn parse(input: ParseStream) -> Result<Self> {
        use Command::*;

        let attrs = input.call(Attribute::parse_outer)?;
        let name = input.parse()?;
        let lookahead = input.lookahead1();
        let case = if lookahead.peek(Token![.]) {
            let _: Token![.] = input.parse()?;

            let content;
            let _ = bracketed!(content in input);
            let children = Punctuated::<Command, Token![,]>::parse_terminated(&content)?
                .iter()
                .cloned()
                .collect();

            Parent {
                name,
                attrs,
                children,
            }
        } else if lookahead.peek(Token![=>]) {
            let _: Token![=>] = input.parse()?;
            let runnable = input.parse()?;

            Runnable {
                name,
                attrs,
                runnable,
            }
        } else {
            return Err(lookahead.error());
        };

        Ok(case)
    }
}

impl ToTokens for Command {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        use Command::*;
        let stream = match self {
            Parent {
                attrs,
                name,
                children,
            } => {
                let enum_name = Ident::new(&format!("{}Command", name), name.span());

                let (decl_cases, rest): (Vec<_>, Vec<(_, Command)>) = children
                    .iter()
                    .map(|cmd| {
                        let case_name = cmd.get_name();
                        let new_cmd = cmd.with_prefix(name);
                        let path_name = new_cmd.get_name();

                        (
                            quote! { #case_name(#path_name) },
                            (quote! { #case_name(cmd) => cmd.run() }, new_cmd),
                        )
                    })
                    .unzip();

                let (run_cases, children): (Vec<_>, Vec<_>) = rest.iter().cloned().unzip();

                quote! {
                    #(#attrs)*
                    #[derive(Clap)]
                    struct #name {
                        #[clap(subcommand)]
                        subcmd: #enum_name,
                    }

                    impl #name {
                        pub fn run(&self) {
                            self.subcmd.run();
                        }
                    }

                    #[derive(Clap)]
                    enum #enum_name {
                        #(#decl_cases),*
                    }

                    impl #enum_name {
                        pub fn run(&self) {
                            use #enum_name::*;

                            match self {
                                #(#run_cases),*
                            }
                        }
                    }

                    #(#children)*
                }
            }
            Runnable {
                attrs,
                name,
                runnable,
            } => quote! {
                    #(#attrs)*
                    #[derive(Clap)]
                    struct #name;

                    impl #name {
                        pub fn run(&self) {
                            #runnable();
                        }
                    }
            },
        };

        tokens.append_all(stream)
    }
}

impl Command {
    fn get_name(&self) -> &Ident {
        use Command::*;

        match self {
            Parent {
                name,
                attrs: _,
                children: _,
            } => name,
            Runnable {
                name,
                attrs: _,
                runnable: _,
            } => name,
        }
    }

    fn with_prefix(&self, prefix: &Ident) -> Command {
        use Command::*;

        match self {
            Parent {
                name,
                attrs,
                children,
            } => Parent {
                name: Ident::new(&format!("{}{}", prefix, name), name.span()),
                attrs: attrs.to_vec(),
                children: children.to_vec(),
            },
            Runnable {
                name,
                attrs,
                runnable,
            } => Runnable {
                name: Ident::new(&format!("{}{}", prefix, name), name.span()),
                attrs: attrs.to_vec(),
                runnable: runnable.clone(),
            },
        }
    }
}
