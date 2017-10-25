use chrono::{TimeZone, DateTime, Local};
use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::server::*;
use serde_json;
use serde_urlencoded;

use assets::ScriptJs;
use mimes::*;
use rendering::render_markdown;
use site::Layout;
use state::State;
use web::{Resource, ResponseFuture};

use super::changes_resource::QueryParameters;

pub struct ArticleResource {
    state: State,
    article_id: i32,
    revision: i32,
    edit: bool,
}

impl ArticleResource {
    pub fn new(state: State, article_id: i32, revision: i32, edit: bool) -> Self {
        Self { state, article_id, revision, edit }
    }
}

pub fn last_updated(article_id: i32, created: &DateTime<Local>, author: Option<&str>) -> String {
    struct Author<'a> {
        author: &'a str,
        history: String,
    }

    #[derive(BartDisplay)]
    #[template_string = "Last updated <a href=\"{{article_history}}\">{{created}}</a>{{#author}} by <a href=\"{{.history}}\">{{.author}}</a>{{/author}}"]
    struct Template<'a> {
        created: &'a str,
        article_history: &'a str,
        author: Option<Author<'a>>,
    }

    Template {
        created: &created.to_rfc2822(),
        article_history: &format!("_changes{}", QueryParameters::default().article_id(Some(article_id)).into_link()),
        author: author.map(|author| Author {
            author: &author,
            history: format!("_changes{}", QueryParameters::default().author(Some(author.to_owned())).into_link()),
        }),
    }.to_string()
}

impl Resource for ArticleResource {
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
        #[template="templates/article.html"]
        struct Template<'a> {
            revision: i32,
            last_updated: Option<&'a str>,

            edit: bool,
            cancel_url: Option<&'a str>,
            title: &'a str,
            raw: &'a str,
            rendered: String,

            script_js_checksum: &'a str,
        }

        let data = self.state.get_article_revision(self.article_id, self.revision)
            .map(|x| x.expect("Data model guarantees that this exists"));
        let head = self.head();

        Box::new(data.join(head)
            .and_then(move |(data, head)| {
                Ok(head
                    .with_body(Layout {
                        base: None, // Hmm, should perhaps accept `base` as argument
                        title: &data.title,
                        body: &Template {
                            revision: data.revision,
                            last_updated: Some(&last_updated(
                                data.article_id,
                                &Local.from_utc_datetime(&data.created),
                                data.author.as_ref().map(|x| &**x)
                            )),
                            edit: self.edit,
                            cancel_url: Some(&data.slug),
                            title: &data.title,
                            raw: &data.body,
                            rendered: render_markdown(&data.body),
                            script_js_checksum: ScriptJs::checksum(),
                        },
                    }.to_string()))
            }))
    }

    fn put(self: Box<Self>, body: hyper::Body, identity: Option<String>) -> ResponseFuture {
        // TODO Check incoming Content-Type

        use futures::Stream;

        #[derive(Deserialize)]
        struct UpdateArticle {
            base_revision: i32,
            title: String,
            body: String,
        }

        #[derive(BartDisplay)]
        #[template="templates/article_contents.html"]
        struct Template<'a> {
            title: &'a str,
            rendered: String,
        }

        #[derive(Serialize)]
        struct PutResponse<'a> {
            slug: &'a str,
            revision: i32,
            title: &'a str,
            rendered: &'a str,
            last_updated: &'a str,
        }

        Box::new(body
            .concat2()
            .map_err(Into::into)
            .and_then(|body| {
                serde_urlencoded::from_bytes(&body)
                    .map_err(Into::into)
            })
            .and_then(move |update: UpdateArticle| {
                self.state.update_article(self.article_id, update.base_revision, update.title, update.body, identity)
            })
            .and_then(|updated| {
                futures::finished(Response::new()
                    .with_status(hyper::StatusCode::Ok)
                    .with_header(ContentType(APPLICATION_JSON.clone()))
                    .with_body(serde_json::to_string(&PutResponse {
                        slug: &updated.slug,
                        revision: updated.revision,
                        title: &updated.title,
                        rendered: &Template {
                            title: &updated.title,
                            rendered: render_markdown(&updated.body),
                        }.to_string(),
                        last_updated: &last_updated(
                            updated.article_id,
                            &Local.from_utc_datetime(&updated.created),
                            updated.author.as_ref().map(|x| &**x)
                        ),
                    }).expect("Should never fail"))
                )
            })
        )
    }
}
