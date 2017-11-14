#![recursion_limit="128"]

#[macro_use] extern crate quote;
#[macro_use] extern crate serde_derive;
extern crate base64;
extern crate proc_macro;
extern crate serde_json;
extern crate serde;
extern crate sha2;
extern crate syn;

use proc_macro::TokenStream;

mod licenses;
mod static_resource;

#[proc_macro_derive(StaticResource, attributes(filename, mime))]
pub fn static_resource(input: TokenStream) -> TokenStream {
    static_resource::static_resource(input)
}

#[proc_macro_derive(Licenses)]
pub fn licenses(input: TokenStream) -> TokenStream {
    licenses::licenses(input)
}
