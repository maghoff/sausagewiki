use chrono;

#[derive(Debug, Queryable)]
pub struct ArticleRevision {
    pub sequence_number: i32,

    pub article_id: i32,
    pub revision: i32,
    pub created: chrono::NaiveDateTime,

    pub slug: String,
    pub title: String,
    pub body: String,

    pub latest: bool,
}
