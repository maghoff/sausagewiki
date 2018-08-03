use diesel;
use schema::article_revisions;

mod query_parameters;
mod resource;
mod scope;

pub use self::query_parameters::QueryParameters;
pub use self::scope::Scope;
pub use self::resource::Resource;

fn apply_query_config<'a>(
    mut query: article_revisions::BoxedQuery<'a, diesel::sqlite::Sqlite>,
    article_id: Option<i32>,
    author: Option<String>,
    limit: i32,
)
    -> article_revisions::BoxedQuery<'a, diesel::sqlite::Sqlite>
{
    use diesel::prelude::*;

    if let Some(article_id) = article_id {
        query = query.filter(article_revisions::article_id.eq(article_id));
    }

    if let Some(author) = author {
        query = query.filter(article_revisions::author.eq(author));
    }

    query.limit(limit as i64 + 1)
}
