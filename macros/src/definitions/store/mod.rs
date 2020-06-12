mod nodes;
mod parse;
mod tokenize;

pub use nodes::Ipc;

mod store_keywords {
    syn::custom_keyword!(command);
    syn::custom_keyword!(query);
    syn::custom_keyword!(into);
}

