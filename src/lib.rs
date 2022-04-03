#![allow(clippy::into_iter_on_ref)]
#![allow(clippy::vec_init_then_push)]
#![recursion_limit = "128"]
// for diesel's infer_schema!

// Diesel causes many warnings of the following kind. I expect this to be
// fixed in a future release of Diesel. Currently used version of Diesel is
// 1.3.0.
#![allow(proc_macro_derive_resolution_fallback)]

#[cfg(test)]
#[macro_use]
extern crate matches;
#[macro_use]
extern crate bart_derive;
#[macro_use]
extern crate codegen;
#[macro_use]
#[allow(clippy::useless_attribute)]
#[allow(deprecated)]
extern crate diesel_infer_schema;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate hyper;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_plain;

extern crate chrono;
extern crate diff;
extern crate futures;
extern crate futures_cpupool;
extern crate percent_encoding;
extern crate pulldown_cmark;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate rand;
extern crate seahash;
extern crate serde;
extern crate serde_json;
extern crate serde_urlencoded;
extern crate slug;
extern crate titlecase;

use std::net::{IpAddr, SocketAddr};

mod assets;
mod build_config;
mod db;
mod merge;
mod mimes;
mod models;
mod rendering;
mod resources;
mod schema;
mod site;
mod state;
mod theme;
mod web;
mod wiki_lookup;

pub fn main(
    db_file: String,
    bind_host: IpAddr,
    bind_port: u16,
    trust_identity: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let db_pool = db::create_pool(db_file)?;
    let cpu_pool = futures_cpupool::CpuPool::new_num_cpus();

    let state = state::State::new(db_pool, cpu_pool);
    let lookup = wiki_lookup::WikiLookup::new(state, trust_identity);

    let server = hyper::server::Http::new()
        .bind(&SocketAddr::new(bind_host, bind_port), move || {
            Ok(site::Site::new(lookup.clone(), trust_identity))
        })?;

    println!("Listening on http://{}", server.local_addr().unwrap());

    server.run()?;

    Ok(())
}
