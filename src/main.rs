#[macro_use] extern crate lazy_static;
extern crate clap;
extern crate sausagewiki;

use std::net::IpAddr;

mod build_config;
use build_config::*;

const DATABASE: &str = "DATABASE";
const TRUST_IDENTITY: &str = "trust-identity";
const ADDRESS: &str = "address";
const PORT: &str = "port";

fn args<'a>() -> clap::ArgMatches<'a> {
    use clap::{App, Arg};

    App::new(PROJECT_NAME)
        .version(VERSION.as_str())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(Arg::with_name(DATABASE)
            .help("Sets the database file to use")
            .required(true))
        .arg(Arg::with_name(PORT)
            .help("Sets the listening port")
            .short("p")
            .long(PORT)
            .default_value("8080")
            .validator(|x| match x.parse::<u16>() {
                Ok(_) => Ok(()),
                Err(_) => Err("Must be an integer in the range [0, 65535]".into())
            })
            .takes_value(true))
        .arg(Arg::with_name(ADDRESS)
            .help("Sets the IP address to bind to")
            .short("a")
            .long(ADDRESS)
            .default_value("127.0.0.1")
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

    const CLAP: &str = "Guaranteed by clap";
    const VALIDATOR: &str = "Guaranteed by clap validator";
    let db_file = args.value_of(DATABASE).expect(CLAP).to_owned();
    let bind_host = args.value_of(ADDRESS).expect(CLAP).parse().expect(VALIDATOR);
    let bind_port = args.value_of(PORT).expect(CLAP).parse().expect(VALIDATOR);

    let trust_identity = args.is_present(TRUST_IDENTITY);

    sausagewiki::main(
        db_file,
        bind_host,
        bind_port,
        trust_identity,
    )
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
