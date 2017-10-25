use chrono::{TimeZone, DateTime, Local};
use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::server::*;

use mimes::*;
use models;
use rendering::render_markdown;
use site::Layout;
use web::{Resource, ResponseFuture};

use super::changes_resource::QueryParameters;
use super::pagination::Pagination;

pub struct ArticleRevisionResource {
    data: models::ArticleRevision,
}

impl ArticleRevisionResource {
    pub fn new(data: models::ArticleRevision) -> Self {
        Self { data }
    }
}

pub fn timestamp_and_author(sequence_number: i32, article_id: i32, created: &DateTime<Local>, author: Option<&str>) -> String {
    struct Author<'a> {
        author: &'a str,
        history: String,
    }

    #[derive(BartDisplay)]
    #[template_string = "<a href=\"{{article_history}}\">{{created}}</a>{{#author}} by <a href=\"{{.history}}\">{{.author}}</a>{{/author}}"]
    struct Template<'a> {
        created: &'a str,
        article_history: &'a str,
        author: Option<Author<'a>>,
    }

    let pagination = Pagination::After(sequence_number - 1);

    Template {
        created: &created.to_rfc2822(),
        article_history: &format!("_changes{}",
            QueryParameters::default()
                .pagination(pagination)
                .article_id(Some(article_id))
                .into_link()
        ),
        author: author.map(|author| Author {
            author: &author,
            history: format!("_changes{}",
                QueryParameters::default()
                    .pagination(pagination)
                    .article_id(Some(article_id))
                    .author(Some(author.to_owned()))
                    .into_link()
            ),
        }),
    }.to_string()
}

impl Resource for ArticleRevisionResource {
    fn allow(&self) -> Vec<hyper::Method> {
        use hyper::Method::*;
        vec![Options, Head, Get, Put]
    }

    fn head(&self) -> ResponseFuture {
        Box::new(futures::finished(Response::new()
            .with_status(hyper::StatusCode::Ok)
            .with_header(ContentType(TEXT_HTML.clone()))
        ))
    }

    fn get(self: Box<Self>) -> ResponseFuture {
        #[derive(BartDisplay)]
        #[template="templates/article_revision.html"]
        struct Template<'a> {
            link_current: &'a str,
            timestamp_and_author: &'a str,

            title: &'a str,
            rendered: String,
        }

        let head = self.head();
        let data = self.data;

        Box::new(head
            .and_then(move |head|
                Ok(head
                    .with_body(Layout {
                        base: Some("../../"), // Hmm, should perhaps accept `base` as argument
                        title: &data.title,
                        body: &Template {
                            link_current: &format!("_by_id/{}", data.article_id),
                            timestamp_and_author: &timestamp_and_author(
                                data.sequence_number,
                                data.article_id,
                                &Local.from_utc_datetime(&data.created),
                                data.author.as_ref().map(|x| &**x)
                            ),
                            title: &data.title,
                            rendered: render_markdown(&data.body),
                        },
                    }.to_string()))
            ))
    }
}
