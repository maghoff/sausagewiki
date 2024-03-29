use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use futures_cpupool::{self, CpuFuture};
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;

use crate::merge;
use crate::models;
use crate::schema::*;
use crate::theme::Theme;

#[derive(Clone)]
pub struct State {
    connection_pool: Pool<ConnectionManager<SqliteConnection>>,
    cpu_pool: futures_cpupool::CpuPool,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub enum SlugLookup {
    Miss,
    Hit { article_id: i32, revision: i32 },
    Redirect(String),
}

#[derive(Insertable)]
#[table_name = "article_revisions"]
struct NewRevision<'a> {
    article_id: i32,
    revision: i32,
    slug: &'a str,
    title: &'a str,
    body: &'a str,
    author: Option<&'a str>,
    latest: bool,
    theme: Theme,
}

#[derive(Debug, PartialEq)]
pub struct RebaseConflict {
    pub base_article: models::ArticleRevisionStub,
    pub title: merge::MergeResult<char>,
    pub body: merge::MergeResult<String>,
    pub theme: Theme,
}

#[derive(Debug, PartialEq)]
enum RebaseResult {
    Clean {
        title: String,
        body: String,
        theme: Theme,
    },
    Conflict(RebaseConflict),
}

pub enum UpdateResult {
    Success(models::ArticleRevision),
    RebaseConflict(RebaseConflict),
}

fn decide_slug(
    conn: &SqliteConnection,
    article_id: i32,
    prev_title: &str,
    title: &str,
    prev_slug: Option<&str>,
) -> Result<String, Error> {
    let base_slug = ::slug::slugify(title);

    if let Some(prev_slug) = prev_slug {
        if prev_slug.is_empty() {
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

    let base_slug = if base_slug.is_empty() {
        "article"
    } else {
        &base_slug
    };

    let mut slug = base_slug.to_owned();
    let mut disambiguator = 1;

    loop {
        let slug_in_use = article_revisions::table
            .filter(article_revisions::article_id.ne(article_id))
            .filter(article_revisions::slug.eq(&slug))
            .filter(article_revisions::latest.eq(true))
            .count()
            .first::<i64>(conn)?
            != 0;

        if !slug_in_use {
            break Ok(slug);
        }

        disambiguator += 1;
        slug = format!("{}-{}", base_slug, disambiguator);
    }
}

struct SyncState<'a> {
    db_connection: &'a diesel::SqliteConnection,
}

impl<'a> SyncState<'a> {
    fn new(db_connection: &diesel::SqliteConnection) -> SyncState {
        SyncState { db_connection }
    }

    pub fn get_article_slug(&self, article_id: i32) -> Result<Option<String>, Error> {
        Ok(article_revisions::table
            .filter(article_revisions::article_id.eq(article_id))
            .filter(article_revisions::latest.eq(true))
            .select(article_revisions::slug)
            .first::<String>(self.db_connection)
            .optional()?)
    }

    pub fn get_article_revision(
        &self,
        article_id: i32,
        revision: i32,
    ) -> Result<Option<models::ArticleRevision>, Error> {
        Ok(article_revisions::table
            .filter(article_revisions::article_id.eq(article_id))
            .filter(article_revisions::revision.eq(revision))
            .first::<models::ArticleRevision>(self.db_connection)
            .optional()?)
    }

    pub fn query_article_revision_stubs<F>(
        &self,
        f: F,
    ) -> Result<Vec<models::ArticleRevisionStub>, Error>
    where
        F: 'static + Send + Sync,
        for<'x> F: FnOnce(
            article_revisions::BoxedQuery<'x, diesel::sqlite::Sqlite>,
        ) -> article_revisions::BoxedQuery<'x, diesel::sqlite::Sqlite>,
    {
        use crate::schema::article_revisions::dsl::*;

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
                theme,
            ))
            .load(self.db_connection)?)
    }

    fn get_article_revision_stub(
        &self,
        article_id: i32,
        revision: i32,
    ) -> Result<Option<models::ArticleRevisionStub>, Error> {
        Ok(self
            .query_article_revision_stubs(move |query| {
                query
                    .filter(article_revisions::article_id.eq(article_id))
                    .filter(article_revisions::revision.eq(revision))
                    .limit(1)
            })?
            .pop())
    }

    pub fn lookup_slug(&self, slug: String) -> Result<SlugLookup, Error> {
        #[derive(Queryable)]
        struct ArticleRevisionStub {
            article_id: i32,
            revision: i32,
            latest: bool,
        }

        self.db_connection.transaction(|| {
            Ok(
                match article_revisions::table
                    .filter(article_revisions::slug.eq(slug))
                    .order(article_revisions::sequence_number.desc())
                    .select((
                        article_revisions::article_id,
                        article_revisions::revision,
                        article_revisions::latest,
                    ))
                    .first::<ArticleRevisionStub>(self.db_connection)
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
                            .first::<String>(self.db_connection)?,
                    ),
                },
            )
        })
    }

    fn rebase_update(
        &self,
        article_id: i32,
        target_base_revision: i32,
        existing_base_revision: i32,
        title: String,
        body: String,
        theme: Theme,
    ) -> Result<RebaseResult, Error> {
        let mut title_a = title;
        let mut body_a = body;
        let mut theme_a = theme;

        // TODO: Improve this implementation.
        // Weakness: If the range of revisions is big, _one_ request from the
        //   client can cause _many_ database requests, cheaply causing lots
        //   of work for the server. Possible attack vector.
        // Weakness: When the range is larger than just one iteration, the
        //   same title and body are retrieved from the database multiple
        //   times. Unnecessary extra copies.

        for revision in existing_base_revision..target_base_revision {
            let mut stored = article_revisions::table
                .filter(article_revisions::article_id.eq(article_id))
                .filter(article_revisions::revision.ge(revision))
                .filter(article_revisions::revision.le(revision + 1))
                .order(article_revisions::revision.asc())
                .select((
                    article_revisions::title,
                    article_revisions::body,
                    article_revisions::theme,
                ))
                .load::<(String, String, Theme)>(self.db_connection)?;

            let (title_b, body_b, theme_b) = stored.pop().expect("Application layer guarantee");
            let (title_o, body_o, theme_o) = stored.pop().expect("Application layer guarantee");

            use crate::merge::MergeResult::*;

            fn merge_themes(a: Theme, o: Theme, b: Theme) -> Theme {
                // Last change wins
                if a != o {
                    a
                } else {
                    b
                }
            }

            let update = {
                let title_merge = merge::merge_chars(&title_a, &title_o, &title_b);
                let body_merge = merge::merge_lines(&body_a, &body_o, &body_b);
                let theme = merge_themes(theme_a, theme_o, theme_b);

                match (title_merge, body_merge) {
                    (Clean(title), Clean(body)) => (title, body, theme),
                    (title_merge, body_merge) => {
                        return Ok(RebaseResult::Conflict(RebaseConflict {
                            base_article: self
                                .get_article_revision_stub(article_id, revision + 1)?
                                .expect("Application layer guarantee"),
                            title: title_merge,
                            body: body_merge.into_strings(),
                            theme,
                        }));
                    }
                }
            };

            title_a = update.0;
            body_a = update.1;
            theme_a = update.2;
        }

        Ok(RebaseResult::Clean {
            title: title_a,
            body: body_a,
            theme: theme_a,
        })
    }

    pub fn update_article(
        &self,
        article_id: i32,
        base_revision: i32,
        title: String,
        body: String,
        author: Option<String>,
        theme: Option<Theme>,
    ) -> Result<UpdateResult, Error> {
        if title.is_empty() {
            return Err("title cannot be empty".into());
        }

        self.db_connection.transaction(|| {
            let (latest_revision, prev_title, prev_slug, prev_theme) = article_revisions::table
                .filter(article_revisions::article_id.eq(article_id))
                .order(article_revisions::revision.desc())
                .select((
                    article_revisions::revision,
                    article_revisions::title,
                    article_revisions::slug,
                    article_revisions::theme,
                ))
                .first::<(i32, String, String, Theme)>(self.db_connection)?;

            // TODO: If this is an historic edit repeated, just respond OK
            // This scheme would make POST idempotent.

            if base_revision > latest_revision {
                return Err("This edit is based on a future version of the article".into());
            }

            let theme = theme.unwrap_or(prev_theme);
            let rebase_result = self.rebase_update(
                article_id,
                latest_revision,
                base_revision,
                title,
                body,
                theme,
            )?;

            let (title, body, theme) = match rebase_result {
                RebaseResult::Clean { title, body, theme } => (title, body, theme),
                RebaseResult::Conflict(x) => return Ok(UpdateResult::RebaseConflict(x)),
            };

            let new_revision = latest_revision + 1;

            let slug = decide_slug(
                self.db_connection,
                article_id,
                &prev_title,
                &title,
                Some(&prev_slug),
            )?;

            diesel::update(
                article_revisions::table
                    .filter(article_revisions::article_id.eq(article_id))
                    .filter(article_revisions::revision.eq(latest_revision)),
            )
            .set(article_revisions::latest.eq(false))
            .execute(self.db_connection)?;

            diesel::insert_into(article_revisions::table)
                .values(&NewRevision {
                    article_id,
                    revision: new_revision,
                    slug: &slug,
                    title: &title,
                    body: &body,
                    author: author.as_deref(),
                    latest: true,
                    theme,
                })
                .execute(self.db_connection)?;

            Ok(UpdateResult::Success(
                article_revisions::table
                    .filter(article_revisions::article_id.eq(article_id))
                    .filter(article_revisions::revision.eq(new_revision))
                    .first::<models::ArticleRevision>(self.db_connection)?,
            ))
        })
    }

    pub fn create_article(
        &self,
        target_slug: Option<String>,
        title: String,
        body: String,
        author: Option<String>,
        theme: Theme,
    ) -> Result<models::ArticleRevision, Error> {
        if title.is_empty() {
            return Err("title cannot be empty".into());
        }

        self.db_connection.transaction(|| {
            #[derive(Insertable)]
            #[table_name = "articles"]
            struct NewArticle {
                id: Option<i32>,
            }

            let article_id = {
                use diesel::expression::sql_literal::sql;
                // Diesel and SQLite are a bit in disagreement for how this should look:
                sql::<diesel::sql_types::Integer>("INSERT INTO articles VALUES (null)")
                    .execute(self.db_connection)?;
                sql::<diesel::sql_types::Integer>("SELECT LAST_INSERT_ROWID()")
                    .load::<i32>(self.db_connection)?
                    .pop()
                    .expect("Statement must evaluate to an integer")
            };

            let slug = decide_slug(
                self.db_connection,
                article_id,
                "",
                &title,
                target_slug.as_deref(),
            )?;

            let new_revision = 1;

            diesel::insert_into(article_revisions::table)
                .values(&NewRevision {
                    article_id,
                    revision: new_revision,
                    slug: &slug,
                    title: &title,
                    body: &body,
                    author: author.as_deref(),
                    latest: true,
                    theme,
                })
                .execute(self.db_connection)?;

            Ok(article_revisions::table
                .filter(article_revisions::article_id.eq(article_id))
                .filter(article_revisions::revision.eq(new_revision))
                .first::<models::ArticleRevision>(self.db_connection)?)
        })
    }

    pub fn search_query(
        &self,
        query_string: String,
        limit: i32,
        offset: i32,
        snippet_size: i32,
    ) -> Result<Vec<models::SearchResult>, Error> {
        use diesel::sql_query;
        use diesel::sql_types::{Integer, Text};

        fn fts_quote(src: &str) -> String {
            format!("\"{}\"", src.replace('\"', "\"\""))
        }

        let words = query_string
            .split_whitespace()
            .map(fts_quote)
            .collect::<Vec<_>>();

        let query = if words.len() > 1 {
            format!("NEAR({})", words.join(" "))
        } else if words.len() == 1 {
            format!("{}*", words[0])
        } else {
            "\"\"".to_owned()
        };

        Ok(
            sql_query(
                "SELECT title, snippet(article_search, 1, '<em>', '</em>', '\u{2026}', ?) AS snippet, slug \
                    FROM article_search \
                    WHERE article_search MATCH ? \
                    ORDER BY rank \
                    LIMIT ? OFFSET ?"
            )
            .bind::<Integer, _>(snippet_size)
            .bind::<Text, _>(query)
            .bind::<Integer, _>(limit)
            .bind::<Integer, _>(offset)
            .load(self.db_connection)?)
    }
}

impl State {
    pub fn new(
        connection_pool: Pool<ConnectionManager<SqliteConnection>>,
        cpu_pool: futures_cpupool::CpuPool,
    ) -> State {
        State {
            connection_pool,
            cpu_pool,
        }
    }

    fn execute<F, T>(&self, f: F) -> CpuFuture<T, Error>
    where
        F: 'static + Sync + Send,
        for<'a> F: FnOnce(SyncState<'a>) -> Result<T, Error>,
        T: 'static + Send,
    {
        let connection_pool = self.connection_pool.clone();

        self.cpu_pool.spawn_fn(move || {
            let db_connection = connection_pool.get()?;

            f(SyncState::new(&*db_connection))
        })
    }

    pub fn get_article_slug(&self, article_id: i32) -> CpuFuture<Option<String>, Error> {
        self.execute(move |state| state.get_article_slug(article_id))
    }

    pub fn get_article_revision(
        &self,
        article_id: i32,
        revision: i32,
    ) -> CpuFuture<Option<models::ArticleRevision>, Error> {
        self.execute(move |state| state.get_article_revision(article_id, revision))
    }

    pub fn query_article_revision_stubs<F>(
        &self,
        f: F,
    ) -> CpuFuture<Vec<models::ArticleRevisionStub>, Error>
    where
        F: 'static + Send + Sync,
        for<'a> F: FnOnce(
            article_revisions::BoxedQuery<'a, diesel::sqlite::Sqlite>,
        ) -> article_revisions::BoxedQuery<'a, diesel::sqlite::Sqlite>,
    {
        self.execute(move |state| state.query_article_revision_stubs(f))
    }

    pub fn get_latest_article_revision_stubs(
        &self,
    ) -> CpuFuture<Vec<models::ArticleRevisionStub>, Error> {
        self.query_article_revision_stubs(|query| {
            query
                .filter(article_revisions::latest.eq(true))
                .order(article_revisions::title.asc())
        })
    }

    pub fn lookup_slug(&self, slug: String) -> CpuFuture<SlugLookup, Error> {
        self.execute(move |state| state.lookup_slug(slug))
    }

    pub fn update_article(
        &self,
        article_id: i32,
        base_revision: i32,
        title: String,
        body: String,
        author: Option<String>,
        theme: Option<Theme>,
    ) -> CpuFuture<UpdateResult, Error> {
        self.execute(move |state| {
            state.update_article(article_id, base_revision, title, body, author, theme)
        })
    }

    pub fn create_article(
        &self,
        target_slug: Option<String>,
        title: String,
        body: String,
        author: Option<String>,
        theme: Theme,
    ) -> CpuFuture<models::ArticleRevision, Error> {
        self.execute(move |state| state.create_article(target_slug, title, body, author, theme))
    }

    pub fn search_query(
        &self,
        query_string: String,
        limit: i32,
        offset: i32,
        snippet_size: i32,
    ) -> CpuFuture<Vec<models::SearchResult>, Error> {
        self.execute(move |state| state.search_query(query_string, limit, offset, snippet_size))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db;

    impl UpdateResult {
        pub fn unwrap(self) -> models::ArticleRevision {
            match self {
                UpdateResult::Success(x) => x,
                _ => panic!("Expected success"),
            }
        }
    }

    macro_rules! init {
        ($state:ident) => {
            let db = db::test_connection();
            let $state = SyncState::new(&db);
        };
    }

    #[test]
    fn get_article_slug() {
        init!(state);
        assert_matches!(state.get_article_slug(0), Ok(None));
    }

    #[test]
    fn create_article() {
        init!(state);
        let article_revision = state
            .create_article(None, "Title".into(), "Body".into(), None, Theme::Cyan)
            .unwrap();
        assert_eq!("title", article_revision.slug);
        assert!(article_revision.latest);
        assert_eq!(Theme::Cyan, article_revision.theme);
    }

    #[test]
    fn create_article_when_empty_slug_then_empty_slug() {
        // Front page gets to keep its empty slug
        init!(state);
        let article_revision = state
            .create_article(
                Some("".into()),
                "Title".into(),
                "Body".into(),
                None,
                Theme::Cyan,
            )
            .unwrap();
        assert_eq!("", article_revision.slug);
    }

    #[test]
    fn update_article() {
        init!(state);

        let article = state
            .create_article(None, "Title".into(), "Body".into(), None, Theme::Cyan)
            .unwrap();

        let new_revision = state
            .update_article(
                article.article_id,
                article.revision,
                article.title.clone(),
                "New body".into(),
                None,
                Some(Theme::BlueGray),
            )
            .unwrap()
            .unwrap();

        assert_eq!(article.article_id, new_revision.article_id);

        // Revision numbers must actually be sequential:
        assert_eq!(article.revision + 1, new_revision.revision);

        assert_eq!(article.title, new_revision.title);

        // Slug must remain unchanged when the title is unchanged:
        assert_eq!(article.slug, new_revision.slug);

        assert_eq!("New body", new_revision.body);
        assert_eq!(Theme::BlueGray, new_revision.theme);
    }

    #[test]
    fn update_article_when_sequential_edits_then_last_wins() {
        init!(state);

        let article = state
            .create_article(None, "Title".into(), "Body".into(), None, Theme::Cyan)
            .unwrap();

        let first_edit = state
            .update_article(
                article.article_id,
                article.revision,
                article.title.clone(),
                "New body".into(),
                None,
                Some(Theme::Blue),
            )
            .unwrap()
            .unwrap();
        let second_edit = state
            .update_article(
                article.article_id,
                first_edit.revision,
                article.title,
                "Newer body".into(),
                None,
                Some(Theme::Amber),
            )
            .unwrap()
            .unwrap();

        assert_eq!("Newer body", second_edit.body);
        assert_eq!(Theme::Amber, second_edit.theme);
    }

    #[test]
    fn update_article_when_edit_conflict_then_merge() {
        init!(state);

        let article = state
            .create_article(None, "Title".into(), "a\nb\nc\n".into(), None, Theme::Cyan)
            .unwrap();

        let first_edit = state
            .update_article(
                article.article_id,
                article.revision,
                article.title.clone(),
                "a\nx\nb\nc\n".into(),
                None,
                Some(Theme::Blue),
            )
            .unwrap()
            .unwrap();
        let second_edit = state
            .update_article(
                article.article_id,
                article.revision,
                article.title.clone(),
                "a\nb\ny\nc\n".into(),
                None,
                Some(Theme::Amber),
            )
            .unwrap()
            .unwrap();

        assert!(article.revision < first_edit.revision);
        assert!(first_edit.revision < second_edit.revision);

        assert_eq!("a\nx\nb\ny\nc\n", second_edit.body);
        assert_eq!(Theme::Amber, second_edit.theme);
    }

    #[test]
    fn update_article_when_edit_conflict_then_rebase_over_multiple_revisions() {
        init!(state);

        let article = state
            .create_article(None, "Title".into(), "a\nb\nc\n".into(), None, Theme::Cyan)
            .unwrap();

        let edit = state
            .update_article(
                article.article_id,
                article.revision,
                article.title.clone(),
                "a\nx1\nb\nc\n".into(),
                None,
                Some(article.theme),
            )
            .unwrap()
            .unwrap();
        let edit = state
            .update_article(
                article.article_id,
                edit.revision,
                article.title.clone(),
                "a\nx1\nx2\nb\nc\n".into(),
                None,
                Some(article.theme),
            )
            .unwrap()
            .unwrap();
        let edit = state
            .update_article(
                article.article_id,
                edit.revision,
                article.title.clone(),
                "a\nx1\nx2\nx3\nb\nc\n".into(),
                None,
                Some(article.theme),
            )
            .unwrap()
            .unwrap();

        let rebase_edit = state
            .update_article(
                article.article_id,
                article.revision,
                article.title.clone(),
                "a\nb\ny\nc\n".into(),
                None,
                Some(article.theme),
            )
            .unwrap()
            .unwrap();

        assert!(article.revision < edit.revision);
        assert!(edit.revision < rebase_edit.revision);

        assert_eq!("a\nx1\nx2\nx3\nb\ny\nc\n", rebase_edit.body);
    }

    #[test]
    fn update_article_when_title_edit_conflict_then_merge_title() {
        init!(state);

        let article = state
            .create_article(None, "titlle".into(), "".into(), None, Theme::Cyan)
            .unwrap();

        let first_edit = state
            .update_article(
                article.article_id,
                article.revision,
                "Titlle".into(),
                article.body.clone(),
                None,
                Some(article.theme),
            )
            .unwrap()
            .unwrap();
        let second_edit = state
            .update_article(
                article.article_id,
                article.revision,
                "title".into(),
                article.body.clone(),
                None,
                Some(article.theme),
            )
            .unwrap()
            .unwrap();

        assert!(article.revision < first_edit.revision);
        assert!(first_edit.revision < second_edit.revision);

        assert_eq!("Title", second_edit.title);
    }

    #[test]
    fn update_article_when_merge_conflict() {
        init!(state);

        let article = state
            .create_article(None, "Title".into(), "a".into(), None, Theme::Cyan)
            .unwrap();

        let first_edit = state
            .update_article(
                article.article_id,
                article.revision,
                article.title.clone(),
                "b".into(),
                None,
                Some(Theme::Blue),
            )
            .unwrap()
            .unwrap();
        let conflict_edit = state
            .update_article(
                article.article_id,
                article.revision,
                article.title.clone(),
                "c".into(),
                None,
                Some(Theme::Amber),
            )
            .unwrap();

        match conflict_edit {
            UpdateResult::Success(..) => panic!("Expected conflict"),
            UpdateResult::RebaseConflict(RebaseConflict {
                base_article,
                title,
                body,
                theme,
            }) => {
                assert_eq!(first_edit.revision, base_article.revision);
                assert_eq!(title, merge::MergeResult::Clean(article.title));
                assert_eq!(
                    body,
                    merge::MergeResult::Conflicted(vec![merge::Output::Conflict(
                        vec!["c"],
                        vec!["a"],
                        vec!["b"]
                    ),])
                    .into_strings()
                );
                assert_eq!(Theme::Amber, theme);
            }
        };
    }

    #[test]
    fn update_article_when_theme_conflict_then_ignore_unchanged() {
        init!(state);

        let article = state
            .create_article(None, "Title".into(), "a\nb\nc\n".into(), None, Theme::Cyan)
            .unwrap();

        let _first_edit = state
            .update_article(
                article.article_id,
                article.revision,
                article.title.clone(),
                "a\nx\nb\nc\n".into(),
                None,
                Some(Theme::Blue),
            )
            .unwrap()
            .unwrap();
        let second_edit = state
            .update_article(
                article.article_id,
                article.revision,
                article.title,
                "a\nb\ny\nc\n".into(),
                None,
                Some(Theme::Cyan),
            )
            .unwrap()
            .unwrap();

        assert_eq!(Theme::Blue, second_edit.theme);
    }

    #[test]
    fn update_article_with_no_given_theme_then_theme_unchanged() {
        init!(state);

        let article = state
            .create_article(None, "Title".into(), "a\nb\nc\n".into(), None, Theme::Cyan)
            .unwrap();

        let edit = state
            .update_article(
                article.article_id,
                article.revision,
                article.title,
                article.body,
                None,
                None,
            )
            .unwrap()
            .unwrap();

        assert_eq!(Theme::Cyan, edit.theme);
    }
}
