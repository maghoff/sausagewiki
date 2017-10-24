use std;

use diesel;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use futures_cpupool::{self, CpuFuture};
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;

use models;
use schema::*;

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

#[derive(Insertable)]
#[table_name="article_revisions"]
struct NewRevision<'a> {
    article_id: i32,
    revision: i32,
    slug: &'a str,
    title: &'a str,
    body: &'a str,
    author: Option<&'a str>,
    latest: bool,
}

fn decide_slug(conn: &SqliteConnection, article_id: i32, prev_title: &str, title: &str, prev_slug: Option<&str>) -> Result<String, Error> {
    let base_slug = ::slug::slugify(title);

    if let Some(prev_slug) = prev_slug {
        if prev_slug == "" {
            // Never give a non-empty slug to the front page
            return Ok(String::new());
        }

        if title == prev_title {
            return Ok(prev_slug.to_owned());
        }

        if base_slug == prev_slug {
            return Ok(base_slug);
        }
    }

    use schema::article_revisions;

    let mut slug = base_slug.clone();
    let mut disambiguator = 1;

    loop {
        let slug_in_use = article_revisions::table
            .filter(article_revisions::article_id.ne(article_id))
            .filter(article_revisions::slug.eq(&slug))
            .filter(article_revisions::latest.eq(true))
            .count()
            .first::<i64>(conn)? != 0;

        if !slug_in_use {
            break Ok(slug);
        }

        disambiguator += 1;
        slug = format!("{}-{}", base_slug, disambiguator);
    }
}

impl State {
    pub fn new(connection_pool: Pool<ConnectionManager<SqliteConnection>>, cpu_pool: futures_cpupool::CpuPool) -> State {
        State {
            connection_pool,
            cpu_pool,
        }
    }

    pub fn get_article_slug(&self, article_id: i32) -> CpuFuture<Option<String>, Error> {
        let connection_pool = self.connection_pool.clone();

        self.cpu_pool.spawn_fn(move || {
            use schema::article_revisions;

            Ok(article_revisions::table
                .filter(article_revisions::article_id.eq(article_id))
                .filter(article_revisions::latest.eq(true))
                .select((article_revisions::slug))
                .first::<String>(&*connection_pool.get()?)
                .optional()?)
        })
    }

    pub fn get_article_revision(&self, article_id: i32, revision: i32) -> CpuFuture<Option<models::ArticleRevision>, Error> {
        let connection_pool = self.connection_pool.clone();

        self.cpu_pool.spawn_fn(move || {
            use schema::article_revisions;

            Ok(article_revisions::table
                .filter(article_revisions::article_id.eq(article_id))
                .filter(article_revisions::revision.eq(revision))
                .first::<models::ArticleRevision>(&*connection_pool.get()?)
                .optional()?)
        })
    }

    pub fn query_article_revision_stubs<F>(&self, f: F) -> CpuFuture<Vec<models::ArticleRevisionStub>, Error>
    where
        F: 'static + Send + Sync,
        for <'a> F:
            FnOnce(article_revisions::BoxedQuery<'a, diesel::sqlite::Sqlite>) ->
                article_revisions::BoxedQuery<'a, diesel::sqlite::Sqlite>,
    {
        let connection_pool = self.connection_pool.clone();

        self.cpu_pool.spawn_fn(move || {
            use schema::article_revisions::dsl::*;

            Ok(f(article_revisions.into_boxed())
                .select((
                    sequence_number,
                    article_id,
                    revision,
                    created,
                    slug,
                    title,
                    latest,
                    author,
                ))
                .load(&*connection_pool.get()?)?
            )
        })
    }

    pub fn get_latest_article_revision_stubs(&self) -> CpuFuture<Vec<models::ArticleRevisionStub>, Error> {
        let connection_pool = self.connection_pool.clone();

        self.cpu_pool.spawn_fn(move || {
            use schema::article_revisions;

            Ok(article_revisions::table
                .filter(article_revisions::latest.eq(true))
                .order(article_revisions::title.asc())
                .select((
                    article_revisions::sequence_number,
                    article_revisions::article_id,
                    article_revisions::revision,
                    article_revisions::created,
                    article_revisions::slug,
                    article_revisions::title,
                    article_revisions::latest,
                    article_revisions::author,
                ))
                .load(&*connection_pool.get()?)?)
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
                    .select((
                        article_revisions::article_id,
                        article_revisions::revision,
                        article_revisions::latest,
                    ))
                    .first::<ArticleRevisionStub>(&*conn)
                    .optional()?
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
                            .select(article_revisions::slug)
                            .first::<String>(&*conn)?
                    )
                })
            })
        })
    }

    pub fn update_article(&self, article_id: i32, base_revision: i32, title: String, body: String, author: Option<String>)
        -> CpuFuture<models::ArticleRevision, Error>
    {
        let connection_pool = self.connection_pool.clone();

        self.cpu_pool.spawn_fn(move || {
            let conn = connection_pool.get()?;

            conn.transaction(|| {
                use schema::article_revisions;

                let (latest_revision, prev_title, prev_slug) = article_revisions::table
                    .filter(article_revisions::article_id.eq(article_id))
                    .order(article_revisions::revision.desc())
                    .select((
                        article_revisions::revision,
                        article_revisions::title,
                        article_revisions::slug,
                    ))
                    .first::<(i32, String, String)>(&*conn)?;

                if latest_revision != base_revision {
                    // TODO: If it is the same edit repeated, just respond OK
                    // TODO: If there is a conflict, transform the edit to work seamlessly
                    unimplemented!("TODO Missing handling of revision conflicts");
                }
                let new_revision = base_revision + 1;

                let slug = decide_slug(&*conn, article_id, &prev_title, &title, Some(&prev_slug))?;

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
                        author: author.as_ref().map(|x| &**x),
                        latest: true,
                    })
                    .into(article_revisions::table)
                    .execute(&*conn)?;

                Ok(article_revisions::table
                    .filter(article_revisions::article_id.eq(article_id))
                    .filter(article_revisions::revision.eq(new_revision))
                    .first::<models::ArticleRevision>(&*conn)?
                )
            })
        })
    }

    pub fn create_article(&self, target_slug: Option<String>, title: String, body: String, author: Option<String>)
        -> CpuFuture<models::ArticleRevision, Error>
    {
        let connection_pool = self.connection_pool.clone();

        self.cpu_pool.spawn_fn(move || {
            let conn = connection_pool.get()?;

            conn.transaction(|| {
                #[derive(Insertable)]
                #[table_name="articles"]
                struct NewArticle {
                    id: Option<i32>
                }

                let article_id = {
                    use diesel::expression::sql_literal::sql;
                    // Diesel and SQLite are a bit in disagreement for how this should look:
                    sql::<(diesel::types::Integer)>("INSERT INTO articles VALUES (null)")
                        .execute(&*conn)?;
                    sql::<(diesel::types::Integer)>("SELECT LAST_INSERT_ROWID()")
                        .load::<i32>(&*conn)?
                        .pop().expect("Statement must evaluate to an integer")
                };

                let slug = decide_slug(&*conn, article_id, "", &title, target_slug.as_ref().map(|x| &**x))?;

                let new_revision = 1;

                diesel::insert(&NewRevision {
                        article_id,
                        revision: new_revision,
                        slug: &slug,
                        title: &title,
                        body: &body,
                        author: author.as_ref().map(|x| &**x),
                        latest: true,
                    })
                    .into(article_revisions::table)
                    .execute(&*conn)?;

                Ok(article_revisions::table
                    .filter(article_revisions::article_id.eq(article_id))
                    .filter(article_revisions::revision.eq(new_revision))
                    .first::<models::ArticleRevision>(&*conn)?
                )
            })
        })
    }
}
