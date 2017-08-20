use std;

use chrono;
use diesel::sqlite::SqliteConnection;

use models;

pub struct State {
    db_connection: SqliteConnection
}

impl State {
    pub fn new(db_connection: SqliteConnection) -> State {
        State { db_connection }
    }

    pub fn find_article_by_slug(&self, slug: &str) -> Result<Option<models::Article>, Box<std::error::Error>> {
        Ok(Some(models::Article {
            id: 0,
            revision: 0,
            created: chrono::Local::now().naive_local(),
            title: slug.to_owned(),
            body: "Look at me!".to_owned(),
        }))
    }
}
