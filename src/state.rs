use std;

use diesel::sqlite::SqliteConnection;

pub struct State {
    db_connection: SqliteConnection
}

impl State {
    pub fn new(db_connection: SqliteConnection) -> State {
        State { db_connection }
    }

    pub fn find_article_by_slug(&self, slug: &str) -> Result<Option<()>, Box<std::error::Error>> {
        Ok(None)
    }
}
