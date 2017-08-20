#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;

extern crate clap;
extern crate hyper;

use std::net::{SocketAddr, IpAddr};

mod db;
mod schema;
mod site;

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
            .takes_value(true))
        .get_matches()
}

fn start_server(bind_host: IpAddr, bind_port: u16) -> Result<(), Box<std::error::Error>> {
    let server =
        hyper::server::Http::new()
            .bind(
                &SocketAddr::new(bind_host, bind_port),
                || Ok(site::Site {})
            )?;

    server.run()?;

    Ok(())
}

fn main() {
    let args = args();

    let db_file = args.value_of("DATABASE").expect("Guaranteed by clap");
    let bind_host = "127.0.0.1".parse().unwrap();
    let bind_port = args.value_of("port").map(|p| p.parse().expect("Port must be an unsigned integer")).unwrap_or(8080);

    let _db_connection = db::connect_database(db_file, true);

    start_server(bind_host, bind_port).unwrap();
}
