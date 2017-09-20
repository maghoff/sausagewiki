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

pub enum SlugLookup {
    Miss,
    Hit {
        article_id: i32,
        revision: i32,
    },
    Redirect(String),
}

impl State {
    pub fn new(connection_pool: Pool<ConnectionManager<SqliteConnection>>, cpu_pool: futures_cpupool::CpuPool) -> State {
        State {
            connection_pool,
            cpu_pool,
        }
    }

    pub fn get_article_revision(&self, article_id: i32, revision: i32) -> CpuFuture<Option<models::ArticleRevision>, Error> {
        let connection_pool = self.connection_pool.clone();

        self.cpu_pool.spawn_fn(move || {
            use schema::article_revisions;

            Ok(article_revisions::table
                .filter(article_revisions::article_id.eq(article_id))
                .filter(article_revisions::revision.eq(revision))
                .limit(1)
                .load::<models::ArticleRevision>(&*connection_pool.get()?)?
                .pop())
        })
    }

    pub fn lookup_slug(&self, slug: String) -> CpuFuture<SlugLookup, Error> {
        #[derive(Queryable)]
        struct ArticleRevisionStub {
            article_id: i32,
            revision: i32,
            latest: bool,
        }

        let connection_pool = self.connection_pool.clone();

        self.cpu_pool.spawn_fn(move || {
            let conn = connection_pool.get()?;

            conn.transaction(|| {
                use schema::article_revisions;

                Ok(match article_revisions::table
                    .filter(article_revisions::slug.eq(slug))
                    .order(article_revisions::sequence_number.desc())
                    .limit(1)
                    .select((
                        article_revisions::article_id,
                        article_revisions::revision,
                        article_revisions::latest,
                    ))
                    .load::<ArticleRevisionStub>(&*conn)?
                    .pop()
                {
                    None => SlugLookup::Miss,
                    Some(ref stub) if stub.latest => SlugLookup::Hit {
                        article_id: stub.article_id,
                        revision: stub.revision,
                    },
                    Some(stub) => SlugLookup::Redirect(
                        article_revisions::table
                            .filter(article_revisions::latest.eq(true))
                            .filter(article_revisions::article_id.eq(stub.article_id))
                            .limit(1)
                            .select(article_revisions::slug)
                            .load::<String>(&*conn)?
                            .pop()
                            .expect("Data model requires this to exist")
                    )
                })
            })
        })
    }

    pub fn update_article(&self, article_id: i32, base_revision: i32, body: String) -> CpuFuture<models::ArticleRevision, Error> {
        let connection_pool = self.connection_pool.clone();

        self.cpu_pool.spawn_fn(move || {
            let conn = connection_pool.get()?;

            conn.transaction(|| {
                use schema::article_revisions;

                // TODO: Get title and slug as parameters to update_article, so we can... update those
                let (latest_revision, title, slug) = article_revisions::table
                    .filter(article_revisions::article_id.eq(article_id))
                    .order(article_revisions::revision.desc())
                    .limit(1)
                    .select((
                        article_revisions::revision,
                        article_revisions::title,
                        article_revisions::slug,
                    ))
                    .load::<(i32, String, String)>(&*conn)?
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
                    slug: &'a str,
                    title: &'a str,
                    body: &'a str,
                    latest: bool,
                }

                diesel::update(
                    article_revisions::table
                        .filter(article_revisions::article_id.eq(article_id))
                        .filter(article_revisions::revision.eq(base_revision))
                )
                    .set(article_revisions::latest.eq(false))
                    .execute(&*conn)?;

                diesel::insert(&NewRevision {
                        article_id,
                        revision: new_revision,
                        slug: &slug,
                        title: &title,
                        body: &body,
                        latest: true,
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
