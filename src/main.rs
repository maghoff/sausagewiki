#![recursion_limit="128"] // for diesel's infer_schema!

#[macro_use] extern crate bart_derive;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;
#[macro_use] extern crate hyper;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate static_resource_derive;

extern crate chrono;
extern crate clap;
extern crate diff;
extern crate futures;
extern crate futures_cpupool;
extern crate percent_encoding;
extern crate pulldown_cmark;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate serde;
extern crate serde_json;
extern crate serde_urlencoded;
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

const DATABASE: &str = "DATABASE";
const TRUST_IDENTITY: &str = "trust-identity";
const ADDRESS: &str = "address";
const PORT: &str = "port";

fn args<'a>() -> clap::ArgMatches<'a> {
    use clap::{App, Arg};

    App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(Arg::with_name(DATABASE)
            .help("Sets the database file to use")
            .required(true))
        .arg(Arg::with_name(PORT)
            .help("Sets the listening port. Defaults to 8080")
            .short("p")
            .long(PORT)
            .validator(|x| match x.parse::<u16>() {
                Ok(_) => Ok(()),
                Err(_) => Err("Must be an integer in the range [0, 65535]".into())
            })
            .takes_value(true))
        .arg(Arg::with_name(ADDRESS)
            .help("Sets the TCP address to bind to. Defaults to 127.0.0.1")
            .short("a")
            .long(ADDRESS)
            .validator(|x| match x.parse::<IpAddr>() {
                Ok(_) => Ok(()),
                Err(_) => Err("Must be a valid IP address".into())
            })
            .takes_value(true))
        .arg(Arg::with_name(TRUST_IDENTITY)
            .help("Trust the value in the X-Identity header to be an \
                authenticated username. This only makes sense when Sausagewiki \
                runs behind a reverse proxy which sets this header.")
            .long(TRUST_IDENTITY))
        .get_matches()
}

fn core_main() -> Result<(), Box<std::error::Error>> {
    let args = args();

    let db_file = args.value_of(DATABASE).expect("Guaranteed by clap").to_owned();
    let bind_host = args.value_of(ADDRESS)
        .map(|p| p.parse().expect("Guaranteed by validator"))
        .unwrap_or_else(|| "127.0.0.1".parse().unwrap());
    let bind_port = args.value_of(PORT)
        .map(|p| p.parse().expect("Guaranteed by validator"))
        .unwrap_or(8080);

    let trust_identity = args.is_present(TRUST_IDENTITY);

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

fn main() {
    match core_main() {
        Ok(()) => (),
        Err(err) => {
            eprintln!("{:#?}", err);
            std::process::exit(1)
        }
    }
}
