#[macro_use] extern crate bart_derive;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate static_resource_derive;

extern crate chrono;
extern crate clap;
extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate pulldown_cmark;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate serde_json;
extern crate serde_urlencoded;

use std::net::SocketAddr;

mod db;
mod models;
mod schema;
mod site;
mod state;
mod web;

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
        .get_matches()
}

fn core_main() -> Result<(), Box<std::error::Error>> {
    let args = args();

    let db_file = args.value_of("DATABASE").expect("Guaranteed by clap").to_owned();
    let bind_host = "127.0.0.1".parse().unwrap();
    let bind_port = args.value_of("port")
        .map(|p| p.parse().expect("Guaranteed by validator"))
        .unwrap_or(8080);

    let db_pool = db::create_pool(db_file)?;
    let cpu_pool = futures_cpupool::CpuPool::new_num_cpus();

    let state = state::State::new(db_pool, cpu_pool);

    let server =
        hyper::server::Http::new()
            .bind(
                &SocketAddr::new(bind_host, bind_port),
                move || Ok(site::Site::new(state.clone()))
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
