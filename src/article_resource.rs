use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::server::*;
use serde_json;
use serde_urlencoded;

use assets::{StyleCss, ScriptJs};
use mimes::*;
use rendering::render_markdown;
use site::Layout;
use state::State;
use web::{Resource, ResponseFuture};

pub struct ArticleResource {
    state: State,
    article_id: i32,
    revision: i32,
}

impl ArticleResource {
    pub fn new(state: State, article_id: i32, revision: i32) -> Self {
        Self { state, article_id, revision }
    }
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
        use chrono::{self, TimeZone, Local};

        #[derive(BartDisplay)]
        #[template="templates/article_revision.html"]
        struct Template<'a> {
            article_id: i32,
            revision: i32,
            created: &'a chrono::DateTime<Local>,

            slug: &'a str,
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
                        title: &data.title,
                        body: &Template {
                            article_id: data.article_id,
                            revision: data.revision,
                            created: &Local.from_utc_datetime(&data.created),
                            slug: &data.slug,
                            title: &data.title,
                            raw: &data.body,
                            rendered: render_markdown(&data.body),
                            script_js_checksum: ScriptJs::checksum(),
                        },
                        style_css_checksum: StyleCss::checksum(),
                    }.to_string()))
            }))
    }

    fn put(self: Box<Self>, body: hyper::Body) -> ResponseFuture {
        // TODO Check incoming Content-Type

        use chrono::{TimeZone, Local};
        use futures::Stream;

        #[derive(Deserialize)]
        struct UpdateArticle {
            base_revision: i32,
            title: String,
            body: String,
        }

        #[derive(BartDisplay)]
        #[template="templates/article_revision_contents.html"]
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
            created: &'a str,
        }

        Box::new(body
            .concat2()
            .map_err(Into::into)
            .and_then(|body| {
                serde_urlencoded::from_bytes(&body)
                    .map_err(Into::into)
            })
            .and_then(move |update: UpdateArticle| {
                self.state.update_article(self.article_id, update.base_revision, update.title, update.body)
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
                        created: &Local.from_utc_datetime(&updated.created).to_string(),
                    }).expect("Should never fail"))
                )
            })
        )
    }
}
