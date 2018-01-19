use diesel::prelude::*;
use diesel::expression::sql_literal::sql;
use diesel::sql_types::*;
use r2d2::{CustomizeConnection, Pool};
use r2d2_diesel::{self, ConnectionManager};

embed_migrations!();

#[derive(Debug)]
struct SqliteInitializer;

use std::ffi::CString;

fn markdown_to_fts(ctx: &::diesel::sqlite::Context) -> CString {
    use rendering;
    CString::new(rendering::render_markdown_for_fts(&ctx.get::<String>(0))).unwrap()
}

impl CustomizeConnection<SqliteConnection, r2d2_diesel::Error> for SqliteInitializer {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), r2d2_diesel::Error> {
        sql::<(Integer)>("PRAGMA foreign_keys = ON")
            .execute(conn)
            .map_err(|x| r2d2_diesel::Error::QueryError(x))?;

        conn.create_scalar_function(
            "markdown_to_fts",
            1,
            true,
            markdown_to_fts,
        ).map_err(|x| r2d2_diesel::Error::QueryError(x))?;

        Ok(())
    }
}

pub fn create_pool<S: Into<String>>(connection_string: S) -> Result<Pool<ConnectionManager<SqliteConnection>>, Box<::std::error::Error>> {
    let manager = ConnectionManager::<SqliteConnection>::new(connection_string);
    let pool = Pool::builder()
        .connection_customizer(Box::new(SqliteInitializer {}))
        .build(manager)?;

    embedded_migrations::run(&*pool.get()?)?;

    Ok(pool)
}

#[cfg(test)]
pub fn test_connection() -> SqliteConnection {
    let mut conn = SqliteConnection::establish(":memory:")
        .expect("SQLite should be able to create an in-memory database");

    SqliteInitializer.on_acquire(&mut conn).unwrap();
    embedded_migrations::run(&conn).unwrap();

    conn
}
