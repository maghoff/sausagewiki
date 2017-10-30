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

    pub author: Option<String>,
}

impl ArticleRevision {
    pub fn link(&self) -> &str {
        if self.slug.is_empty() {
            "."
        } else {
            &self.slug
        }
    }
}

#[derive(Debug, Queryable)]
pub struct ArticleRevisionStub {
    pub sequence_number: i32,

    pub article_id: i32,
    pub revision: i32,
    pub created: chrono::NaiveDateTime,

    pub slug: String,
    pub title: String,

    pub latest: bool,

    pub author: Option<String>,
}

#[derive(Debug, Queryable, Serialize)]
pub struct SearchResult {
    pub title: String,
    pub snippet: String,
    pub slug: String,
}
