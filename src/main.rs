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

use std::net::SocketAddr;

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

fn args<'a>() -> clap::ArgMatches<'a> {
    use clap::{App, Arg};

    App::new("sausagewiki")
        .about("A wiki engine")
        .arg(Arg::with_name("DATABASE")
            .help("Sets the database file to use")
            .required(true))
        .arg(Arg::with_name("port")
            .help("Sets the listening port")
            .short("p")
            .long("port")
            .validator(|x| match x.parse::<u16>() {
                Ok(_) => Ok(()),
                Err(_) => Err("Must be an integer in the range [0, 65535]".to_owned())
            })
            .takes_value(true))
        .arg(Arg::with_name("trust_identity")
            .help("Trust the value in the X-Identity header to be an \
                authenticated username. This only makes sense when Sausagewiki \
                runs behind a reverse proxy which sets this header.")
            .long("trust_identity"))
        .get_matches()
}

fn core_main() -> Result<(), Box<std::error::Error>> {
    let args = args();

    let db_file = args.value_of("DATABASE").expect("Guaranteed by clap").to_owned();
    let bind_host = "127.0.0.1".parse().unwrap();
    let bind_port = args.value_of("port")
        .map(|p| p.parse().expect("Guaranteed by validator"))
        .unwrap_or(8080);

    let trust_identity = args.is_present("trust_identity");

    let db_pool = db::create_pool(db_file)?;
    let cpu_pool = futures_cpupool::CpuPool::new_num_cpus();

    let state = state::State::new(db_pool, cpu_pool);
    let lookup = wiki_lookup::WikiLookup::new(state);

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
