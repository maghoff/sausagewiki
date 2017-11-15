#![recursion_limit="128"] // for diesel's infer_schema!

#[cfg(test)] #[macro_use] extern crate matches;

#[macro_use] extern crate bart_derive;
#[macro_use] extern crate codegen;
#[macro_use] extern crate diesel_codegen;
#[macro_use] extern crate diesel;
#[macro_use] extern crate hyper;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate maplit;
#[macro_use] extern crate serde_derive;

extern crate chrono;
extern crate diff;
extern crate futures_cpupool;
extern crate futures;
extern crate percent_encoding;
extern crate pulldown_cmark;
extern crate r2d2_diesel;
extern crate r2d2;
extern crate serde_json;
extern crate serde_urlencoded;
extern crate serde;
extern crate slug;
extern crate titlecase;

use std::net::{IpAddr, SocketAddr};

mod assets;
mod db;
mod mimes;
mod models;
mod rendering;
mod resources;
mod schema;
mod site;
mod state;
mod web;
mod wiki_lookup;

pub fn main(db_file: String, bind_host: IpAddr, bind_port: u16, trust_identity: bool) -> Result<(), Box<std::error::Error>> {
    let db_pool = db::create_pool(db_file)?;
    let cpu_pool = futures_cpupool::CpuPool::new_num_cpus();

    let state = state::State::new(db_pool, cpu_pool);
    let lookup = wiki_lookup::WikiLookup::new(state, trust_identity);

    let server =
        hyper::server::Http::new()
            .bind(
                &SocketAddr::new(bind_host, bind_port),
                move || Ok(site::Site::new(lookup.clone(), trust_identity))
            )?;

    println!("Listening on http://{}", server.local_addr().unwrap());

    server.run()?;

    Ok(())
}
