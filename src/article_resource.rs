use futures::{self, Future};
use hyper;
use hyper::header::ContentType;
use hyper::mime;
use hyper::server::*;
use serde_json;
use serde_urlencoded;

use assets::{StyleCss, ScriptJs};
use models;
use site::Layout;
use state::State;
use web::Resource;

lazy_static! {
    static ref TEXT_HTML: mime::Mime = "text/html;charset=utf-8".parse().unwrap();
    static ref APPLICATION_JSON: mime::Mime = "application/json".parse().unwrap();
}

fn render_markdown(src: &str) -> String {
    use pulldown_cmark::Event;

    struct EscapeHtml<'a, I: Iterator<Item=Event<'a>>> {
        inner: I,
    }

    impl<'a, I: Iterator<Item=Event<'a>>> EscapeHtml<'a, I> {
        fn new(inner: I) -> EscapeHtml<'a, I> {
            EscapeHtml { inner }
        }
    }

    impl<'a, I: Iterator<Item=Event<'a>>> Iterator for EscapeHtml<'a, I> {
        type Item = Event<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            use pulldown_cmark::Event::{Text, Html, InlineHtml};

            match self.inner.next() {
                Some(Html(x)) => Some(Text(x)),
                Some(InlineHtml(x)) => Some(Text(x)),
                x => x
            }
        }
    }

    use pulldown_cmark::{Parser, html};

    let p = EscapeHtml::new(Parser::new(src));
    let mut buf = String::new();
    html::push_html(&mut buf, p);
    buf
}

pub struct ArticleResource {
    state: State,
    data: models::ArticleRevision,
}

impl ArticleResource {
    pub fn new(state: State, data: models::ArticleRevision) -> Self {
        Self { state, data }
    }
}

impl Resource for ArticleResource {
    fn allow(&self) -> Vec<hyper::Method> {
        use hyper::Method::*;
        vec![Options, Head, Get, Put]
    }

    fn head(&self) -> futures::BoxFuture<Response, Box<::std::error::Error + Send + Sync>> {
        futures::finished(Response::new()
            .with_status(hyper::StatusCode::Ok)
            .with_header(ContentType(TEXT_HTML.clone()))
        ).boxed()
    }

    fn get(self: Box<Self>) -> futures::BoxFuture<Response, Box<::std::error::Error + Send + Sync>> {
        use chrono::{self, TimeZone, Local};

        #[derive(BartDisplay)]
        #[template="templates/article_revision.html"]
        struct Template<'a> {
            article_id: i32,
            revision: i32,
            created: &'a chrono::DateTime<Local>,

            title: &'a str,
            raw: &'a str,
            rendered: String,

            script_js_checksum: &'a str,
        }

        self.head().map(move |head|
            head
                .with_body(Layout {
                    title: &self.data.title,
                    body: &Template {
                        article_id: self.data.article_id,
                        revision: self.data.revision,
                        created: &Local.from_utc_datetime(&self.data.created),
                        title: &self.data.title,
                        raw: &self.data.body,
                        rendered: render_markdown(&self.data.body),
                        script_js_checksum: ScriptJs::checksum(),
                    },
                    style_css_checksum: StyleCss::checksum(),
                }.to_string())
        ).boxed()
    }

    fn put(self: Box<Self>, body: hyper::Body) ->
        futures::BoxFuture<Response, Box<::std::error::Error + Send + Sync>>
    {
        // TODO Check incoming Content-Type

        use chrono::{TimeZone, Local};
        use futures::Stream;

        #[derive(Deserialize)]
        struct UpdateArticle {
            base_revision: i32,
            body: String,
        }

        #[derive(Serialize)]
        struct PutResponse<'a> {
            revision: i32,
            rendered: &'a str,
            created: &'a str,
        }

        body
            .concat2()
            .map_err(Into::into)
            .and_then(|body| {
                serde_urlencoded::from_bytes(&body)
                    .map_err(Into::into)
            })
            .and_then(move |update: UpdateArticle| {
                self.state.update_article(self.data.article_id, update.base_revision, update.body)
            })
            .and_then(|updated| {
                futures::finished(Response::new()
                    .with_status(hyper::StatusCode::Ok)
                    .with_header(ContentType(APPLICATION_JSON.clone()))
                    .with_body(serde_json::to_string(&PutResponse {
                        revision: updated.revision,
                        rendered: &render_markdown(&updated.body),
                        created: &Local.from_utc_datetime(&updated.created).to_string(),
                    }).expect("Should never fail"))
                )
            })
            .boxed()
    }
}
