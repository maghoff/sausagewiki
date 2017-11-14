#![recursion_limit="128"]

#[macro_use] extern crate quote;

extern crate base64;
extern crate proc_macro;
extern crate sha2;
extern crate syn;

use proc_macro::TokenStream;

mod static_resource;

#[proc_macro_derive(StaticResource, attributes(filename, mime))]
pub fn static_resource(input: TokenStream) -> TokenStream {
    static_resource::static_resource(input)
}
