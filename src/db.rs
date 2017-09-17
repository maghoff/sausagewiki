use diesel::prelude::*;
use diesel::expression::sql_literal::sql;
use diesel::types::*;
use r2d2::{Config, CustomizeConnection, Pool};
use r2d2_diesel::{self, ConnectionManager};

embed_migrations!();

#[derive(Debug)]
struct SqliteInitializer;

impl CustomizeConnection<SqliteConnection, r2d2_diesel::Error> for SqliteInitializer {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), r2d2_diesel::Error> {
        sql::<(Integer)>("PRAGMA foreign_keys = ON")
            .execute(conn)
            .and(Ok(()))
            .map_err(|x| r2d2_diesel::Error::QueryError(x))
    }
}

pub fn create_pool(connection_string: String) -> Result<Pool<ConnectionManager<SqliteConnection>>, Box<::std::error::Error>> {
    let config = Config::builder()
        .connection_customizer(Box::new(SqliteInitializer {}))
        .build();
    let manager = ConnectionManager::<SqliteConnection>::new(connection_string);

    let pool = Pool::new(config, manager)?;

    embedded_migrations::run(&*pool.get()?)?;

    Ok(pool)
}
