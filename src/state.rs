use std;

use diesel;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use futures_cpupool::{self, CpuFuture};
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;

use models;

#[derive(Clone)]
pub struct State {
    connection_pool: Pool<ConnectionManager<SqliteConnection>>,
    cpu_pool: futures_cpupool::CpuPool,
}

pub type Error = Box<std::error::Error + Send + Sync>;

impl State {
    pub fn new(connection_pool: Pool<ConnectionManager<SqliteConnection>>, cpu_pool: futures_cpupool::CpuPool) -> State {
        State {
            connection_pool,
            cpu_pool,
        }
    }

    pub fn get_article_revision_by_id(&self, article_id: i32) -> CpuFuture<Option<models::ArticleRevision>, Error> {
        let connection_pool = self.connection_pool.clone();

        self.cpu_pool.spawn_fn(move || {
            use schema::article_revisions;

            Ok(article_revisions::table
                .filter(article_revisions::article_id.eq(article_id))
                .order(article_revisions::revision.desc())
                .limit(1)
                .load::<models::ArticleRevision>(&*connection_pool.get()?)?
                .pop())
        })
    }

    pub fn update_article(&self, article_id: i32, base_revision: i32, body: String) -> CpuFuture<models::ArticleRevision, Error> {
        let connection_pool = self.connection_pool.clone();

        self.cpu_pool.spawn_fn(move || {
            let conn = connection_pool.get()?;

            conn.transaction(|| {
                use schema::article_revisions;

                let (latest_revision, title) = article_revisions::table
                    .filter(article_revisions::article_id.eq(article_id))
                    .order(article_revisions::revision.desc())
                    .limit(1)
                    .select((article_revisions::revision, article_revisions::title))
                    .load::<(i32, String)>(&*conn)?
                    .pop()
                    .unwrap_or_else(|| unimplemented!("TODO Missing an error type"));

                if latest_revision != base_revision {
                    // TODO: If it is the same edit repeated, just respond OK
                    // TODO: If there is a conflict, transform the edit to work seamlessly
                    unimplemented!("TODO Missing handling of revision conflicts");
                }
                let new_revision = base_revision + 1;

                #[derive(Insertable)]
                #[table_name="article_revisions"]
                struct NewRevision<'a> {
                    article_id: i32,
                    revision: i32,
                    title: &'a str,
                    body: &'a str,
                }

                diesel::insert(&NewRevision {
                        article_id,
                        revision: new_revision,
                        title: &title,
                        body: &body,
                    })
                    .into(article_revisions::table)
                    .execute(&*conn)?;

                Ok(article_revisions::table
                    .filter(article_revisions::article_id.eq(article_id))
                    .filter(article_revisions::revision.eq(new_revision))
                    .load::<models::ArticleRevision>(&*conn)?
                    .pop()
                    .expect("We just inserted this row!")
                )
            })
        })
    }
}
