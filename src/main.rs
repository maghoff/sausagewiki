#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;

extern crate clap;

mod db;
mod schema;

fn args<'a>() -> clap::ArgMatches<'a> {
    use clap::{App, Arg};

    App::new("sausagewiki")
        .about("A wiki engine")
        .arg(Arg::with_name("DATABASE")
            .help("Sets the database file to use")
            .required(true)
            .index(1))
        .get_matches()
}

fn main() {
    let args = args();

    let db_file = args.value_of("DATABASE").expect("Guaranteed by clap");

    let _db_connection = db::connect_database(db_file, true);
}
