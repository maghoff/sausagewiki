use diesel::prelude::*;
use diesel::expression::sql_literal::sql;
use diesel::sql_types::*;
use r2d2::{CustomizeConnection, Pool};
use r2d2_diesel::{self, ConnectionManager};

use rendering;
use theme;

embed_migrations!();

#[derive(Debug)]
struct SqliteInitializer;

#[allow(dead_code)]
pub mod sqlfunc {
    use diesel::sql_types::Text;
    sql_function!(fn markdown_to_fts(text: Text) -> Text);
    sql_function!(fn theme_from_str_hash(text: Text) -> Text);
}

impl CustomizeConnection<SqliteConnection, r2d2_diesel::Error> for SqliteInitializer {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), r2d2_diesel::Error> {
        sql::<(Integer)>("PRAGMA foreign_keys = ON")
            .execute(conn)
            .map_err(|x| r2d2_diesel::Error::QueryError(x))?;

        sqlfunc::markdown_to_fts::register_impl(
            conn,
            |text: String| rendering::render_markdown_for_fts(&text)
        ).map_err(|x| r2d2_diesel::Error::QueryError(x))?;

        sqlfunc::theme_from_str_hash::register_impl(
            conn,
            |title: String| theme::theme_from_str_hash(&title)
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

#[cfg(test)]
mod test {
    use super::*;

    use diesel::sql_query;

    #[test]
    fn markdown_to_fts() {
        let conn = test_connection();

        #[derive(QueryableByName, PartialEq, Eq, Debug)]
        struct Row { #[sql_type = "Text"] text: String }

        let res = sql_query("SELECT markdown_to_fts('[link](url)') as text")
            .load::<Row>(&conn)
            .unwrap();

        let expected = rendering::render_markdown_for_fts("[link](url)");

        assert_eq!(expected, res[0].text);
    }

    #[test]
    fn theme_from_str_hash() {
        let conn = test_connection();

        #[derive(QueryableByName, PartialEq, Eq, Debug)]
        struct Row { #[sql_type = "Text"] theme: theme::Theme }

        let res = sql_query("SELECT theme_from_str_hash('Bartefjes') as theme")
            .load::<Row>(&conn)
            .unwrap();

        let expected = theme::theme_from_str_hash("Bartefjes");

        assert_eq!(expected, res[0].theme);
    }
}
