//! This crate provides a derive macro for `ConfigType`.
#![recursion_limit = "256"]
mod attrs;
mod config_type;
mod item_enum;
mod utils;
use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn config_type(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::Item);
    let output = config_type::define_config_type(&input);
    TokenStream::from(output)
}
