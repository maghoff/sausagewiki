use chrono;

use crate::theme::Theme;

fn slug_link(slug: &str) -> &str {
    if slug.is_empty() {
        "."
    } else {
        slug
    }
}

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

    pub theme: Theme,
}

impl ArticleRevision {
    pub fn link(&self) -> &str { slug_link(&self.slug) }
}

#[derive(Debug, PartialEq, Queryable)]
pub struct ArticleRevisionStub {
    pub sequence_number: i32,

    pub article_id: i32,
    pub revision: i32,
    pub created: chrono::NaiveDateTime,

    pub slug: String,
    pub title: String,

    pub latest: bool,

    pub author: Option<String>,

    pub theme: Theme,
}

impl ArticleRevisionStub {
    pub fn link(&self) -> &str { slug_link(&self.slug) }
}

use diesel::sql_types::Text;
#[derive(Debug, QueryableByName, Serialize)]
pub struct SearchResult {
    #[sql_type = "Text"]
    pub title: String,

    #[sql_type = "Text"]
    pub snippet: String,

    #[sql_type = "Text"]
    pub slug: String,
}

impl SearchResult {
    pub fn link(&self) -> &str { slug_link(&self.slug) }
}
