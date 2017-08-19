use diesel::Connection;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::expression::sql_literal::sql;
use diesel::types::*;

embed_migrations!();

pub fn connect_database(connection_string: &str, run_migrations: bool) -> SqliteConnection {
    let connection = SqliteConnection::establish(connection_string)
        .expect(&format!("Error connecting to database at {}", connection_string));

    // Integer is a dummy placeholder. Compiling fails when passing ().
    sql::<(Integer)>("PRAGMA foreign_keys = ON")
        .execute(&connection)
        .expect("Should be able to enable foreign_keys");

    if run_migrations {
        embedded_migrations::run(&connection).unwrap();
    }

    connection
}
