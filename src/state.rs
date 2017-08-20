use std;

use chrono;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;

use models;

pub struct State {
    db_connection: SqliteConnection
}

impl State {
    pub fn new(db_connection: SqliteConnection) -> State {
        State { db_connection }
    }

    pub fn get_article_revision_by_slug(&self, slug: &str) -> Result<Option<models::ArticleRevision>, Box<std::error::Error>> {
        Ok(Some(models::ArticleRevision {
            article_id: 0,
            revision: 0,
            created: chrono::Local::now().naive_local(),
            title: slug.to_owned(),
            body: "Look at me!".to_owned(),
        }))
    }

    pub fn get_article_revision_by_id(&self, article_id: i32) -> Result<Option<models::ArticleRevision>, Box<std::error::Error>> {
        use schema::article_revisions;

        Ok(article_revisions::table
            .filter(article_revisions::article_id.eq(article_id))
            .order(article_revisions::revision.desc())
            .limit(1)
            .load::<models::ArticleRevision>(&self.db_connection)?
            .pop())
    }
}
