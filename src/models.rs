use chrono;

#[derive(BartDisplay, Debug, Queryable)]
#[template="templates/article.html"]
pub struct Article {
    pub id: i32,
    pub revision: i32,
    pub created: chrono::NaiveDateTime,

    pub title: String,
    pub body: String,
}
