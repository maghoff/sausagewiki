use std;

use chrono;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;

use models;

#[derive(Clone)]
pub struct State {
    connection_pool: Pool<ConnectionManager<SqliteConnection>>
}

#[derive(Deserialize)]
pub struct UpdateArticle {
    base_revision: i32,
    body: String,
}

pub type Error = Box<std::error::Error + Send + Sync>;

impl State {
    pub fn new(connection_pool: Pool<ConnectionManager<SqliteConnection>>) -> State {
        State { connection_pool }
    }

    pub fn get_article_revision_by_slug(&self, slug: &str) -> Result<Option<models::ArticleRevision>, Error> {
        Ok(Some(models::ArticleRevision {
            article_id: 0,
            revision: 0,
            created: chrono::Local::now().naive_local(),
            title: slug.to_owned(),
            body: "Look at me!".to_owned(),
        }))
    }

    pub fn get_article_revision_by_id(&self, article_id: i32) -> Result<Option<models::ArticleRevision>, Error> {
        use schema::article_revisions;

        Ok(article_revisions::table
            .filter(article_revisions::article_id.eq(article_id))
            .order(article_revisions::revision.desc())
            .limit(1)
            .load::<models::ArticleRevision>(&*self.connection_pool.get()?)?
            .pop())
    }
}
