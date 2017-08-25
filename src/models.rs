use chrono;

#[derive(BartDisplay, Clone, Debug, Queryable)]
#[template="templates/article_revision.html"]
pub struct ArticleRevision {
    pub article_id: i32,
    pub revision: i32,
    pub created: chrono::NaiveDateTime,

    pub title: String,
    pub body: String,
}
